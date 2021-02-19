extern crate tokio;
extern crate bincode;

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

#[cfg(test)]
use tokio::runtime::Runtime;

pub struct SplitLogFile
{
    pub head_path: String,
    pub data_path: String,

    pub head_file: File,
    pub data_file: File,
    
    pub off_head: u64,
    pub off_data: u64,
}
#[allow(dead_code)]
pub struct EventData
{
    pub header: Header,
    pub data: Bytes,
    pub digest: Bytes,
}

impl SplitLogFile {
    async fn open_file(cfg: &impl ConfigStorage, path: &String) -> File {
        match cfg.log_temp() {
            true => OpenOptions::new().read(true).write(true).create_new(true).create(true).open(path.clone()).await.unwrap(),
               _ => OpenOptions::new().read(true).write(true).append(true).create(true).open(path.clone()).await.unwrap(),
        }
    }

    async fn new(cfg: &impl ConfigStorage, key: &impl ChainKey) -> SplitLogFile {
        let path_head = format!("{}/{}.head", cfg.log_path(), key.to_key_str());
        let path_data = format!("{}/{}.data", cfg.log_path(), key.to_key_str());

        let ret = SplitLogFile {
            head_path: path_head.clone(),
            data_path: path_data.clone(),

            head_file: SplitLogFile::open_file(cfg, &path_head).await,
            data_file: SplitLogFile::open_file(cfg, &path_data).await,
            
            off_head: 0,
            off_data: 0,
        };

        if cfg.log_temp() {
            std::fs::remove_file(path_head).ok();
            std::fs::remove_file(path_data).ok();
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
        let size_head = self.head_file.read_u64().await.ok()?;
        let mut buff_head = BytesMut::with_capacity(size_head as usize);
        self.head_file.read_buf(&mut buff_head).await.ok()?;
        let buff_head = buff_head.freeze();

        let ret: Header = bincode::deserialize(&buff_head).ok()?;

        self.off_head = self.off_head + size_head;
        self.off_data = self.off_data + ret.size_data + ret.size_digest;

        Some(ret)
    }

    pub async fn write(&mut self, header: &mut Header, data: &[u8], digest: &[u8]) -> Result<usize>
    {
        header.size_data = data.len() as u64;
        header.off_data = self.off_data;
        header.size_digest = digest.len() as u64;
        header.off_digest = self.off_data + data.len() as u64;

        let buff_header = bincode::serialize(&header).unwrap();
        
        self.data_file.seek(SeekFrom::Start(header.off_data)).await?;
        self.data_file.write_all(data).await?;
        self.data_file.seek(SeekFrom::Start(header.off_digest)).await?;
        self.data_file.write_all(digest).await?;
        
        self.head_file.write_u64(buff_header.len() as u64).await?;
        self.head_file.write_all(buff_header.as_slice()).await?;
        
        self.off_head = self.off_head + buff_header.len() as u64;
        self.off_data = self.off_data + header.size_data + header.size_digest;

        Ok(((std::mem::size_of::<u64>() as u64) + (buff_header.len() as u64) + header.size_data + header.size_digest) as usize)
    }

    #[allow(dead_code)]
    pub async fn load(&mut self, header: &Header) -> Result<EventData> {
        let mut buff_data = BytesMut::with_capacity(header.size_data as usize);
        let mut buff_digest = BytesMut::with_capacity(header.size_digest as usize);

        self.data_file.seek(SeekFrom::Start(header.off_data)).await?;
        self.data_file.read_buf(&mut buff_data).await?;
        self.data_file.seek(SeekFrom::Start(header.off_digest)).await?;
        self.data_file.read_buf(&mut buff_digest).await?;

        Ok(
            EventData {
                header: header.clone(),
                data: buff_data.freeze(),
                digest: buff_digest.freeze(),
            }
        )
    }
}

pub struct RedoLogProtected {
    file: SplitLogFile,
    entries: VecDeque<Header>,
}

impl RedoLogProtected
{
    async fn new(cfg: &impl ConfigStorage, key: &impl ChainKey) -> Result<RedoLogProtected> {
        let mut ret = RedoLogProtected {
            file: SplitLogFile::new(cfg, key).await,
            entries: VecDeque::new(),
        };

        ret.file.read_all(&mut ret.entries).await;

        Ok(ret)
    }

    async fn write(&mut self, header: &mut Header, data: &[u8], digest: &[u8]) -> Result<usize> {
        let ret = self.file.write(header, data, digest).await;
        self.entries.push_back(header.clone());
        ret
    }

    async fn load(&mut self, header: &Header) -> Result<EventData> {
        self.file.load(header).await
    }

    fn pop(&mut self) -> Option<Header> {
        self.entries.pop_front()
    }
}

pub struct RedoLog {
    #[allow(unused_variables)]
    inside: Arc<Mutex<RedoLogProtected>>,
}

impl RedoLog
{
    #[allow(dead_code)]
    pub async fn new(cfg: &impl ConfigStorage, key: &impl ChainKey) -> Result<RedoLog> {
        Result::Ok(
            RedoLog {
                inside: Arc::new(Mutex::new(RedoLogProtected::new(cfg, key).await?))
            }
        )
    }

    #[allow(dead_code)]
    pub async fn write(&mut self, header: &mut Header, data: &[u8], digest: &[u8]) -> Result<usize> {
        let mut lock = self.inside.lock().await;
        lock.write(header, data, digest).await
    }

    #[allow(dead_code)]
    pub async fn pop(&mut self) -> Option<Header> {
        let mut lock = self.inside.lock().await;
        lock.pop()
    }

    #[allow(dead_code)]
    pub async fn load(&mut self, header: &Header) -> Result<EventData> {
        let mut lock = self.inside.lock().await;
        lock.load(&header).await
    }
}

#[test]
fn test_redo_log() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {    
        let mut rl = RedoLog::new(&mock_test_config(), &mock_test_chain_key()).await.expect("Failed to load the redo log");
        
        let mut mock_head = Header::default();
        mock_head.key = "blah".to_string();

        let mock_digest = vec![0; 100];
        let mock_data = vec![1; 10];

        rl.write(&mut mock_head, mock_data.as_slice(), mock_digest.as_slice()).await.expect("Failed to write the object");

        let read_header = rl.pop().await.expect("Failed to read mocked data");
        assert_eq!(read_header.key, mock_head.key);

        let evt = rl.load(&read_header).await.expect("Failed to load the event record");
        assert_eq!(vec![1; 10], evt.data);
    });
}