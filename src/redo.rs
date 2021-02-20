extern crate tokio;
extern crate bincode;
extern crate fxhash;

use super::conf::*;
use super::chain::*;
use super::header::*;

use std::{collections::VecDeque, io::SeekFrom};
use std::sync::Arc;
use tokio::{fs::File, fs::OpenOptions, io::{AsyncReadExt, AsyncWriteExt, AsyncSeekExt}};
use tokio::sync::Mutex;
use tokio::io::Result;
use bytes::BytesMut;
use bytes::Bytes;
use fxhash::{FxHashMap};

#[cfg(test)]
use tokio::runtime::Runtime;

pub struct EventData
{
    pub header: Header,
    pub data: Bytes,
    pub digest: Bytes,
}

struct LogFile
{
    pub log_path: String,
    pub log_file: File,    
    pub log_off: u64,
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
            index: FxHashMap::default(),
        };

        if temp_file {
            let _ = std::fs::remove_file(path_log);
        }

        ret
    }

    async fn read_all(&mut self, to: &mut VecDeque<Header>) {
        while let Some(head) = self.read_once().await {
            to.push_back(head);
        }
    }

    async fn read_once(&mut self) -> Option<Header>
    {
        // Read the header
        let size_head = self.log_file.read_u32().await.ok()?;
        let mut buff_head = BytesMut::with_capacity(size_head as usize);
        self.log_file.read_buf(&mut buff_head).await.ok()?;
        let buff_head = buff_head.freeze();

        // Skip the data
        let size_data = self.log_file.read_u32().await.ok()?;
        self.log_file.seek(SeekFrom::Current(size_data as i64)).await.ok()?;

        // Skip the digest
        let size_digest = self.log_file.read_u32().await.ok()?;
        self.log_file.seek(SeekFrom::Current(size_digest as i64)).await.ok();

        let header: Header = bincode::deserialize(&buff_head).ok()?;
        
        self.index.insert(header.index(), self.log_off);

        self.log_off = self.log_off + (size_head + size_data + size_digest) as u64;
        Some(header)
    }

    async fn write(&mut self, header: &Header, data: Bytes, digest: Bytes) -> Result<()>
    {
        let data_len = data.len() as u32;
        let digest_len = digest.len() as u32;
        let buff_header = bincode::serialize(header).unwrap();
        let buff_header_len = buff_header.len() as u32;
        
        self.log_file.seek(SeekFrom::Start(self.log_off)).await?;
        self.log_file.write_u32(buff_header_len).await?;
        self.log_file.write_all(buff_header.as_slice()).await?;
        self.log_file.write_u32(data_len).await?;
        self.log_file.write_all(&data[..]).await?;
        self.log_file.write_u32(digest_len).await?;
        self.log_file.write_all(&digest[..]).await?;

        self.index.insert(header.index(), self.log_off);

        self.log_off = self.log_off + (buff_header_len + data_len + digest_len) as u64;

        Ok(())
    }

    async fn load(&mut self, header: &Header) -> Option<EventData> {
        let off_entry = self.index.get(&header.index())?.clone();

        // Skip the header
        self.log_file.seek(SeekFrom::Start(off_entry)).await.ok()?;
        let size_head = self.log_file.read_u32().await.ok()?;
        self.log_file.seek(SeekFrom::Current(size_head as i64)).await.ok()?;

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
                header: header.clone(),
                data: buff_data.freeze(),
                digest: buff_digest.freeze(),
            }
        )
    }

    fn move_log_file(&mut self, new_path: &String) -> Result<()> {
        std::fs::rename(self.log_path.clone(), new_path)?;
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
    pub data: Bytes,
    pub digest: Bytes,
}

impl DeferredWrite {
    pub fn new(header: Header, data: Bytes, digest: Bytes) -> DeferredWrite {
        DeferredWrite {
            header: header,
            data: data,
            digest: digest,
        }
    }
}

struct FlippedLogFile {
    log_file: LogFile,
    deferred: VecDeque<DeferredWrite>,
}

impl FlippedLogFile
{
    #[allow(dead_code)]
    pub async fn write(&mut self, header: Header, data: Bytes, digest: Bytes) -> Result<()> {
        let _ = self.log_file.write(&header, data, digest).await?;
        Ok(())
    }

    async fn flush(&mut self) -> Result<()> {
        self.log_file.flush().await
    }

    async fn truncate(&mut self) -> Result<()> {
        self.log_file.truncate().await?;
        self.deferred.clear();
        Ok(())
    }
}

struct RedoLogProtected {
    log_temp: bool,
    log_path: String,
    log_file: LogFile,
    flip: Option<Arc<Mutex<FlippedLogFile>>>,
    entries: VecDeque<Header>,
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
        };

        ret.log_file.read_all(&mut ret.entries).await;

        Ok(ret)
    }

    async fn write(&mut self, header: Header, data: Bytes, digest: Bytes) -> Result<()> {
        let deferred_write: Option<DeferredWrite> = match &self.flip {
            Some(_) => Some(
                DeferredWrite::new(header.clone(), data.clone(), digest.clone())
            ),
            _ => None,
        };

        let _ = self.log_file.write(&header, data, digest).await?;
        self.entries.push_back(header);

        match deferred_write {
            Some(itm) => {
                if let Some(flip) = &mut self.flip
                {
                    let mut lock = flip.lock().await;
                    lock.deferred.push_back(itm);
                }
            },
            _ => {}
        }

        Ok(())
    }

    async fn begin_flip(&mut self) -> Option<Arc<Mutex<FlippedLogFile>>> {
        match self.flip
        {
            None => {
                let path_flip = format!("{}.flip", self.log_path);

                let flip = FlippedLogFile {
                    log_file: LogFile::new(self.log_temp, path_flip).await,
                    deferred: VecDeque::new(),
                };
                let flip = Arc::new(Mutex::new(flip));
                
                self.flip = Some(flip.clone());

                Some(flip)
            },
            Some(_) => None,
        }
    }

    async fn end_flip(&mut self, flip: &mut FlippedLogFile) -> Result<()> {
        match &self.flip
        {
            Some(_) =>
            {
                let mut new_log_file = flip.log_file.copy().await?;

                while let Some(d) = flip.deferred.pop_front() {
                    new_log_file.write(&d.header, d.data, d.digest).await?;
                }
                if self.log_temp == false {
                    new_log_file.move_log_file(&self.log_path)?;
                }
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

    async fn load(&mut self, header: &Header) -> Option<EventData> {
        self.log_file.load(header).await
    }

    fn pop(&mut self) -> Option<Header> {
        self.entries.pop_front()
    }

    async fn flush(&mut self) -> Result<()> {
        self.log_file.flush().await?;
        if let Some(flip) = &mut self.flip
        {
            let mut lock = flip.lock().await;
            let _ = lock.flush().await?;
        }
        Ok(())
    }

    async fn truncate(&mut self) -> Result<()> {
        self.log_file.truncate().await?;
        if let Some(flip) = &mut self.flip
        {
            let mut lock = flip.lock().await;
            lock.truncate().await?;
        }
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
    pub async fn write(&mut self, header: Header, data: Bytes, digest: Bytes) -> Result<()> {
        let mut lock = self.inside.lock().await;
        lock.write(header, data, digest).await
    }

    #[allow(dead_code)]
    pub async fn pop(&mut self) -> Option<Header> {
        let mut lock = self.inside.lock().await;
        lock.pop()
    }

    #[allow(dead_code)]
    pub async fn load(&mut self, header: &Header) -> Option<EventData> {
        let mut lock = self.inside.lock().await;
        lock.load(&header).await
    }

    #[allow(dead_code)]
    pub async fn truncate(&mut self) -> Result<()> {
        let mut lock = self.inside.lock().await;
        lock.truncate().await
    }

    #[allow(dead_code)]
    pub async fn flush(&mut self) -> Result<()> {
        let mut lock = self.inside.lock().await;
        lock.flush().await
    }

    #[allow(dead_code)]
    async fn begin_flip(&mut self) -> Option<Arc<Mutex<FlippedLogFile>>> {
        let mut lock = self.inside.lock().await;
        lock.begin_flip().await
    }
    
    #[allow(dead_code)]
    async fn end_flip(&mut self, flip: &mut FlippedLogFile) -> Result<()> {
        let mut lock = self.inside.lock().await;
        lock.end_flip(flip).await
    }

    #[allow(dead_code)]
    fn log_path(&self) -> String {
        self.log_path.clone()
    }
}

#[test]
fn test_redo_log_intra() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let mock_key = DiscreteChainKey::default().with_name("test_obj".to_string());
        let mut rl = RedoLog::new(&mock_test_config(), &mock_key).await.expect("Failed to load the redo log");
        
        let mut mock_head = Header::default();
        mock_head.key = "blah".to_string();

        let mock_digest = Bytes::from(vec![0; 100]);
        let mock_data = Bytes::from(vec![1; 10]);

        rl.write(mock_head.clone(), mock_data, mock_digest).await.expect("Failed to write the object");

        let read_header = rl.pop().await.expect("Failed to read mocked data");
        assert_eq!(read_header.key, mock_head.key);

        let evt = rl.load(&read_header).await.expect("Failed to load the event record");
        assert_eq!(vec![1; 10], evt.data);
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
            
        let mut mock_head = Header::default();
        mock_head.key = "blah".to_string();

        {
            let mut rl = RedoLog::new(&mock_cfg, &mock_chain_key).await.expect("Failed to load the redo log");
            let _ = rl.truncate().await;

            let mock_digest = Bytes::from(vec![0; 100]);
            let mock_data = Bytes::from(vec![1; 10]);

            rl.write(mock_head.clone(), mock_data, mock_digest).await.expect("Failed to write the object");
        }

        {
            let mut rl = RedoLog::new(&mock_cfg, &mock_chain_key).await.expect("Failed to load the redo log");

            let read_header = rl.pop().await.expect("Failed to read mocked data");
            assert_eq!(read_header.key, mock_head.key);

            let evt = rl.load(&read_header).await.expect("Failed to load the event record");
            assert_eq!(vec![1; 10], evt.data);

            let _ = std::fs::remove_file(rl.log_path());
        }
    });
}