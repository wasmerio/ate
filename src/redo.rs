extern crate tokio;
extern crate bincode;
extern crate fxhash;

use super::conf::*;
use super::chain::*;
use super::header::*;

use async_trait::async_trait;
#[allow(unused_imports)]
use std::{collections::VecDeque, io::SeekFrom, ops::DerefMut};
use std::sync::Arc;
#[allow(unused_imports)]
use tokio::{fs::File, fs::OpenOptions, io::{AsyncReadExt, AsyncWriteExt, AsyncSeekExt}, time::sleep, time::Duration};
use tokio::sync::Mutex;
use tokio::io::Result;
use bytes::BytesMut;
use bytes::Bytes;
use fxhash::{FxHashMap};

#[cfg(test)]
use tokio::runtime::Runtime;

#[derive(Clone, Debug)]
pub struct EventData
{
    pub index: HeaderIndex,
    pub meta: Bytes,
    pub data: Bytes,
    pub digest: Bytes,
}

#[derive(Clone, Debug)]
pub struct HeaderMeta
{
    pub header: Header,
    pub meta: Bytes
}

struct LogFile
{
    pub log_path: String,
    pub log_file: File,    
    pub log_off: u64,
    pub log_temp: bool,
    pub index: FxHashMap<HeaderIndex, u64>,
}

impl LogFile {
    async fn copy(&self) -> Result<LogFile>
    {
        let mut copy_of_index: FxHashMap<HeaderIndex, u64> = FxHashMap::default();
        for (key, value) in &self.index {
            copy_of_index.insert(key.clone(), value.clone());
        }

        Ok(
            LogFile {
                log_path: self.log_path.clone(),
                log_file: self.log_file.try_clone().await?,
                log_off: self.log_off,
                log_temp: self.log_temp,
                index: copy_of_index,
            }
        )
    }

    async fn new(temp_file: bool, path_log: String) -> LogFile {
        let log_file = match temp_file {
            true => OpenOptions::new().read(true).write(true).create_new(true).create(true).open(path_log.clone()).await.unwrap(),
               _ => OpenOptions::new().read(true).write(true).append(true).create(true).open(path_log.clone()).await.unwrap(),
        };

        let ret = LogFile {
            log_path: path_log.clone(),
            log_file: log_file,
            log_off: 0,
            log_temp: temp_file,
            index: FxHashMap::default(),
        };

        if temp_file {
            let _ = std::fs::remove_file(path_log);
        }

        ret
    }

    async fn read_all(&mut self, to: &mut VecDeque<HeaderMeta>) {
        while let Some(head) = self.read_once().await {
            to.push_back(head);
        }
    }

    async fn read_once(&mut self) -> Option<HeaderMeta>
    {
        // Read the header
        let size_head = self.log_file.read_u32().await.ok()?;
        let mut buff_head = BytesMut::with_capacity(size_head as usize);
        self.log_file.read_buf(&mut buff_head).await.ok()?;
        let buff_head = buff_head.freeze();

        // Read the metadata
        let size_meta = self.log_file.read_u32().await.ok()?;
        let mut buff_meta = BytesMut::with_capacity(size_meta as usize);
        self.log_file.read_buf(&mut buff_meta).await.ok()?;
        let buff_meta = buff_meta.freeze();

        // Skip the data
        let size_data = self.log_file.read_u32().await.ok()?;
        self.log_file.seek(SeekFrom::Current(size_data as i64)).await.ok()?;

        // Skip the digest
        let size_digest = self.log_file.read_u32().await.ok()?;
        self.log_file.seek(SeekFrom::Current(size_digest as i64)).await.ok();

        let header: Header = bincode::deserialize(&buff_head).ok()?;
        
        self.index.insert(header.index(), self.log_off);

        self.log_off = self.log_file.seek(SeekFrom::Current(0)).await.ok()?;

        Some(HeaderMeta {
            header: header,
            meta: buff_meta,
        })
    }

    async fn write(&mut self, header: &Header, meta: Bytes, data: Bytes, digest: Bytes) -> Result<()>
    {
        let meta_len = meta.len() as u32;
        let data_len = data.len() as u32;
        let digest_len = digest.len() as u32;
        let buff_header = bincode::serialize(header).unwrap();
        let buff_header_len = buff_header.len() as u32;
        
        self.log_file.seek(SeekFrom::Start(self.log_off)).await?;
        self.log_file.write_u32(buff_header_len).await?;
        self.log_file.write_all(buff_header.as_slice()).await?;
        self.log_file.write_u32(meta_len).await?;
        self.log_file.write_all(&meta[..]).await?;
        self.log_file.write_u32(data_len).await?;
        self.log_file.write_all(&data[..]).await?;
        self.log_file.write_u32(digest_len).await?;
        self.log_file.write_all(&digest[..]).await?;

        self.index.insert(header.index(), self.log_off);

        self.log_off = self.log_file.seek(SeekFrom::Current(0)).await?;

        Ok(())
    }

    async fn load(&mut self, idx: &HeaderIndex) -> Option<EventData> {
        let off_entry = self.index.get(idx)?.clone();

        // Skip the header
        self.log_file.seek(SeekFrom::Start(off_entry)).await.ok()?;
        let size_head = self.log_file.read_u32().await.ok()?;
        self.log_file.seek(SeekFrom::Current(size_head as i64)).await.ok()?;

        // Read the metadata
        let size_meta = self.log_file.read_u32().await.ok()?;
        let mut buff_meta = BytesMut::with_capacity(size_meta as usize);
        self.log_file.read_buf(&mut buff_meta).await.ok()?;

        // Read the data
        let size_data = self.log_file.read_u32().await.ok()?;
        let mut buff_data = BytesMut::with_capacity(size_data as usize);
        self.log_file.read_buf(&mut buff_data).await.ok()?;

        // Read the digest
        let size_digest = self.log_file.read_u32().await.ok()?;
        let mut buff_digest = BytesMut::with_capacity(size_digest as usize);
        self.log_file.read_buf(&mut buff_digest).await.ok()?;

        Some(
            EventData {
                index: idx.clone(),
                meta: buff_meta.freeze(),
                data: buff_data.freeze(),
                digest: buff_digest.freeze(),
            }
        )
    }

    fn move_log_file(&mut self, new_path: &String) -> Result<()> {
        if self.log_temp == false {
            std::fs::rename(self.log_path.clone(), new_path)?;
        }
        self.log_path = new_path.clone();
        Ok(())
    }

    async fn flush(&mut self) -> Result<()> {
        self.log_file.sync_all().await?;
        Ok(())
    }

    async fn truncate(&mut self) -> Result<()> {
        self.log_file.set_len(0).await?;
        self.log_file.sync_all().await?;
        self.log_off = 0;
        self.index.clear();
        Ok(())
    }
}

struct DeferredWrite {
    pub header: Header,
    pub meta: Bytes,
    pub data: Bytes,
    pub digest: Bytes,
}

impl DeferredWrite {
    pub fn new(header: Header, meta: Bytes, data: Bytes, digest: Bytes) -> DeferredWrite {
        DeferredWrite {
            header: header,
            meta: meta,
            data: data,
            digest: digest,
        }
    }
}

#[async_trait]
pub trait LogWritable {
    async fn write(&mut self, header: Header, meta: Bytes, data: Bytes, digest: Bytes) -> Result<()>;
    async fn flush(&mut self) -> Result<()>;
}

struct FlippedLogFileProtected {
    log_file: LogFile,
}


#[async_trait]
impl LogWritable for FlippedLogFileProtected
{
    #[allow(dead_code)]
    async fn write(&mut self, header: Header, meta: Bytes, data: Bytes, digest: Bytes) -> Result<()> {
        let _ = self.log_file.write(&header, meta, data, digest).await?;
        Ok(())
    }

    async fn flush(&mut self) -> Result<()> {
        self.log_file.flush().await
    }
}

impl FlippedLogFileProtected
{
    async fn truncate(&mut self) -> Result<()> {
        self.log_file.truncate().await?;
        Ok(())
    }

    async fn copy_log_file(&mut self) -> Result<LogFile> {
        let new_log_file = self.log_file.copy().await?;
        Ok(new_log_file)
    }
}

struct FlippedLogFile {
    inside: Arc<Mutex<FlippedLogFileProtected>>,
}

impl FlippedLogFile
{
    #[allow(dead_code)]
    async fn truncate(&mut self) -> Result<()> {
        let mut lock = self.inside.lock().await;
        lock.truncate().await
    }

    async fn copy_log_file(&self) -> Result<LogFile> {
        let mut lock = self.inside.lock().await;
        lock.copy_log_file().await
    }
}

#[async_trait]
impl LogWritable for FlippedLogFile
{
    #[allow(dead_code)]
    async fn write(&mut self, header: Header, meta: Bytes, data: Bytes, digest: Bytes) -> Result<()> {
        let mut lock = self.inside.lock().await;
        lock.write(header, meta, data, digest).await
    }

    async fn flush(&mut self) -> Result<()> {
        let mut lock = self.inside.lock().await;
        lock.flush().await
    }
}

struct RedoLogFlipProtected {
    deferred: Vec<DeferredWrite>,
}

struct RedoLogProtected {
    log_temp: bool,
    log_path: String,
    log_file: LogFile,
    flip: Option<RedoLogFlipProtected>,
    entries: VecDeque<HeaderMeta>,
    orphans: FxHashMap<HeaderIndex, EventData>
}

impl RedoLogProtected
{
    async fn new(cfg: &impl ConfigStorage, path_log: String) -> Result<RedoLogProtected> {
        let mut ret = RedoLogProtected {
            log_temp: cfg.log_temp(),
            log_path: path_log.clone(),
            log_file: LogFile::new(cfg.log_temp(), path_log.clone()).await,
            flip: None,
            entries: VecDeque::new(),
            orphans: FxHashMap::default(),
        };

        ret.log_file.read_all(&mut ret.entries).await;

        Ok(ret)
    }

    async fn write(&mut self, header: Header, meta: Bytes, data: Bytes, digest: Bytes) -> Result<()> {
        let deferred_write: Option<DeferredWrite> = match &self.flip {
            Some(_) => Some(
                DeferredWrite::new(header.clone(), meta.clone(), data.clone(), digest.clone())
            ),
            _ => None,
        };

        let _ = self.log_file.write(&header, meta.clone(), data, digest).await?;
        self.entries.push_back(
            HeaderMeta {
                header: header,
                meta: meta,
            }
        );

        match deferred_write {
            Some(itm) => {
                if let Some(flip) = &mut self.flip
                {
                    flip.deferred.push(itm);
                }
            },
            _ => {}
        }

        Ok(())
    }

    async fn begin_flip(&mut self) -> Option<FlippedLogFile> {
        match self.flip
        {
            None => {
                let path_flip = format!("{}.flip", self.log_path);

                let mut flip = FlippedLogFile {
                    inside: Arc::new(Mutex::new(FlippedLogFileProtected {
                        log_file: LogFile::new(self.log_temp, path_flip).await,
                    })),
                };
                flip.truncate().await.ok()?;
                
                self.flip = Some(RedoLogFlipProtected {
                    deferred: Vec::new(),
                });

                let mut new_orphans = Vec::new();
                for head in &self.entries {
                    new_orphans.push(head.header.index());
                }
                for idx in new_orphans {
                    if let Some(evt) = self.load(&idx).await {
                        self.orphans.insert(idx, evt);
                    }
                }

                Some(flip)
            },
            Some(_) => None,
        }
    }

    async fn end_flip(&mut self, flip: FlippedLogFile) -> Result<()> {
        match &self.flip
        {
            Some(inside) =>
            {
                let mut new_log_file = flip.copy_log_file().await?;
                for d in &inside.deferred {
                    new_log_file.write(&d.header, d.meta.clone(), d.data.clone(), d.digest.clone()).await?;
                }
                new_log_file.move_log_file(&self.log_path)?;
                self.log_file = new_log_file;
                self.flip = None;
                Ok(())
            },
            None =>
            {
                Ok(())
            }
        }
    }

    async fn load(&mut self, idx: &HeaderIndex) -> Option<EventData> {
        match self.log_file.load(idx).await {
            Some(data) => {
                self.orphans.remove(idx);
                Some(data)
            },
            None => {
                let data = self.orphans.get(idx)?;
                Some(data.clone())
            }
        }
    }

    fn pop(&mut self) -> Option<HeaderMeta> {
        self.entries.pop_front()
    }

    async fn flush(&mut self) -> Result<()> {
        self.log_file.flush().await?;
        Ok(())
    }

    async fn truncate(&mut self) -> Result<()> {
        self.log_file.truncate().await?;
        self.flip = None;
        self.entries.clear();
        Ok(())
    }
}

pub struct RedoLog {
    inside: Arc<Mutex<RedoLogProtected>>,
    log_path: String,
}

impl RedoLog
{
    #[allow(dead_code)]
    pub async fn new(cfg: &impl ConfigStorage, key: &impl ChainKey) -> Result<RedoLog> {
        let _ = std::fs::create_dir_all(cfg.log_path());

        let path_log = format!("{}/{}.log", cfg.log_path(), key.to_key_str());

        Result::Ok(
            RedoLog {
                inside: Arc::new(Mutex::new(RedoLogProtected::new(cfg, path_log.clone()).await?)),
                log_path: path_log,
            }
        )
    }

    #[allow(dead_code)]
    pub async fn pop(&mut self) -> Option<HeaderMeta> {
        let mut lock = self.inside.lock().await;
        lock.pop()
    }

    #[allow(dead_code)]
    pub async fn load(&mut self, idx: &HeaderIndex) -> Option<EventData> {
        let mut lock = self.inside.lock().await;
        lock.load(idx).await
    }

    #[allow(dead_code)]
    pub async fn truncate(&mut self) -> Result<()> {
        let mut lock = self.inside.lock().await;
        lock.truncate().await
    }

    #[allow(dead_code)]
    async fn begin_flip(&mut self) -> Option<FlippedLogFile> {
        let mut lock = self.inside.lock().await;
        lock.begin_flip().await
    }
    
    #[allow(dead_code)]
    async fn end_flip(&mut self, flip: FlippedLogFile) -> Result<()> {
        let mut lock = self.inside.lock().await;
        lock.end_flip(flip).await
    }

    #[allow(dead_code)]
    fn log_path(&self) -> String {
        self.log_path.clone()
    }
}

#[async_trait]
impl LogWritable for RedoLog
{
    #[allow(dead_code)]
    async fn write(&mut self, header: Header, meta: Bytes, data: Bytes, digest: Bytes) -> Result<()> {
        let mut lock = self.inside.lock().await;
        lock.write(header, meta, data, digest).await
    }

    #[allow(dead_code)]
    async fn flush(&mut self) -> Result<()> {
        let mut lock = self.inside.lock().await;
        lock.flush().await
    }
}

/* 
TESTS 
*/

#[cfg(test)]
async fn test_write_data(log: &mut dyn LogWritable, key: &'static str, data: Vec<u8>)
{
    // Write some data to the flipped buffer
    let mut mock_head = Header::default();
    mock_head.key = key.to_string();
    let mock_meta = Bytes::from(vec![255; 50]);
    let mock_digest = Bytes::from(vec![0; 100]);
    let mock_data = Bytes::from(data);

    log.write(mock_head, mock_meta, mock_data, mock_digest).await.expect("Failed to write the object");
}

#[cfg(test)]
async fn test_read_data(log: &mut RedoLog, key: &'static str, test_data: Vec<u8>)
{
    let read_header = log.pop().await.expect("Failed to read mocked data");
    assert_eq!(read_header.header.key, key.to_string());
    let evt = log.load(&read_header.header.index()).await.expect(format!("Failed to load the event record for {}", key).as_str());
    assert_eq!(test_data, evt.data);
}

#[cfg(test)]
async fn test_no_read(log: &mut RedoLog)
{
    match log.pop().await {
        Some(_) => panic!("Should not have been anymore data!"),
        _ => {}
    }
}

#[test]
fn test_redo_log_intra() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        // Create the redo log
        println!("test_redo_log_intra - creating the redo log");
        let mock_key = DiscreteChainKey::default().with_name("test_obj".to_string());
        let mut rl = RedoLog::new(&mock_test_config(), &mock_key).await.expect("Failed to load the redo log");
        let _ = rl.truncate().await;

        // Test that its empty
        println!("test_redo_log_intra - confirming its empty");
        test_no_read(&mut rl).await;
        
        // Push some mocked data to it
        println!("test_redo_log_intra - writing test data to log - blah1");
        let _ = test_write_data(&mut rl, "blah1", vec![1; 10]).await;

        // Begin an operation to flip the redo log
        println!("test_redo_log_intra - beginning the flip operation");
        let mut flip = rl.begin_flip().await.unwrap();
        
        // Write some data to the redo log and the backing redo log
        println!("test_redo_log_intra - writing test data to flip - blah2");
        let _ = test_write_data(&mut flip, "blah2", vec![2; 10]).await;
        println!("test_redo_log_intra - writing test data to log - blah3");
        let _ = test_write_data(&mut rl, "blah3", vec![3; 10]).await;
        println!("test_redo_log_intra - writing test data to log - blah4");
        let _ = test_write_data(&mut rl, "blah4", vec![4; 10]).await;
        
        // Check that the correct data is read
        println!("test_redo_log_intra - testing read result of blah1");
        let _ = test_read_data(&mut rl, "blah1", vec![1; 10]).await;
        println!("test_redo_log_intra - testing read result of blah3");
        let _ = test_read_data(&mut rl, "blah3", vec![3; 10]).await;
        
        // End the flip operation
        println!("test_redo_log_intra - finishing the flip operation");
        rl.end_flip(flip).await.expect("Failed to end the flip operation");

        // Check that the correct data is read (blah2 should not be returned as its a part of the earlier redo log after the flip - compacting)
        println!("test_redo_log_intra - testing read result of blah4");
        let _ = test_read_data(&mut rl, "blah4", vec![4; 10]).await;
        test_no_read(&mut rl).await;

        // Write some data to the redo log and the backing redo log
        println!("test_redo_log_intra - confirming no more data");
        test_no_read(&mut rl).await;
        println!("test_redo_log_intra - writing test data to log - blah4");
        let _ = test_write_data(&mut rl, "blah5", vec![5; 10]).await;

        // Read the test data again
        println!("test_redo_log_intra - testing read result of blah4");
        let _ = test_read_data(&mut rl, "blah5", vec![5; 10]).await;
        println!("test_redo_log_intra - confirming no more data");
        test_no_read(&mut rl).await;
    });
}

#[test]
fn test_redo_log_inter() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let mock_cfg = mock_test_config()
            .with_log_temp(false);

        let mock_chain_key = DiscreteChainKey::default()
            .with_name("test_inter".to_string());
            
        {
            // Open the log once for writing
            println!("test_redo_log_inter - creating the redo log");
            let mut rl = RedoLog::new(&mock_cfg, &mock_chain_key).await.expect("Failed to load the redo log");
            let _ = rl.truncate().await;

            // Test that its empty
            println!("test_redo_log_inter - confirming no more data");
            test_no_read(&mut rl).await;

            // Push some mocked data to it
            println!("test_redo_log_inter - writing test data to log - blah1");
            let _ = test_write_data(&mut rl, "blah1", vec![1; 10]).await;

            // Begin an operation to flip the redo log
            println!("test_redo_log_inter - beginning the flip operation");
            let mut flip = rl.begin_flip().await.unwrap();

            // Write some data to the redo log and the backing redo log
            println!("test_redo_log_inter - writing test data to flip - blah2");
            let _ = test_write_data(&mut flip, "blah2", vec![2; 10]).await;
            println!("test_redo_log_inter - writing test data to log - blah3");
            let _ = test_write_data(&mut rl, "blah3", vec![3; 10]).await;
            
            // End the flip operation
            println!("test_redo_log_inter - finishing the flip operation");
            rl.end_flip(flip).await.expect("Failed to end the flip operation");

            // Write some more data
            println!("test_redo_log_inter - writing test data to log - blah4");
            let _ = test_write_data(&mut rl, "blah4", vec![4; 10]).await;

            // Test reading all the data to the end (excluding the new records)
            println!("test_redo_log_inter - testing read result of blah1");
            let _ = test_read_data(&mut rl, "blah1", vec![1; 10]).await;
            println!("test_redo_log_inter - testing read result of blah3");

            let _ = test_read_data(&mut rl, "blah3", vec![3; 10]).await;
            println!("test_redo_log_inter - testing read result of blah4");
            let _ = test_read_data(&mut rl, "blah4", vec![4; 10]).await;
            println!("test_redo_log_inter - confirming no more data");
            test_no_read(&mut rl).await;

            println!("test_redo_log_inter - closing redo log");
        }

        {
            // Open it up again which should check that it loads data properly
            println!("test_redo_log_intra - reopening the redo log");
            let mut rl = RedoLog::new(&mock_cfg, &mock_chain_key).await.expect("Failed to load the redo log");

            // Check that the correct data is read
            println!("test_redo_log_inter - testing read result of blah2");
            let _ = test_read_data(&mut rl, "blah2", vec![2; 10]).await;
            println!("test_redo_log_inter - testing read result of blah3");
            let _ = test_read_data(&mut rl, "blah3", vec![3; 10]).await;
            println!("test_redo_log_inter - testing read result of blah4");
            let _ = test_read_data(&mut rl, "blah4", vec![4; 10]).await;
            println!("test_redo_log_inter - confirming no more data");
            test_no_read(&mut rl).await;

            // Do some final cleanup
            println!("test_redo_log_inter - removing test files");
            let _ = std::fs::remove_file(rl.log_path());
        }
    });
}