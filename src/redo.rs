extern crate tokio;

use super::conf::*;
use super::chain::*;

use std::io::SeekFrom;
use std::sync::Arc;
use tokio::{fs::File, io::{AsyncReadExt, AsyncWriteExt, AsyncSeekExt}};
use tokio::sync::Mutex;
use tokio::io::Result;
use tokio::io::Error;

#[cfg(test)]
use tokio::runtime::Runtime;
struct SplitLogFileOffsets {
    pub offs: u64,
    pub head: u64,
    pub data: u64,
}

pub struct SplitLogFile {
    pub offs: File,
    pub head: File,
    pub data: File,
}

impl SplitLogFile {
    async fn new(cfg: &impl ConfigStorage, key: &impl ChainKey) -> SplitLogFile {
        let path_offs = format!("{}/{}.offs", cfg.log_path(), key.to_key_str());
        let path_head = format!("{}/{}.head", cfg.log_path(), key.to_key_str());
        let path_data = format!("{}/{}.data", cfg.log_path(), key.to_key_str());

        SplitLogFile {
            offs: tokio::fs::File::create(path_offs).await.unwrap(),
            head: tokio::fs::File::create(path_head).await.unwrap(),
            data: tokio::fs::File::create(path_data).await.unwrap(),
        }
    }

    async fn offsets(&mut self) -> Result<SplitLogFileOffsets> {
        Ok(
            SplitLogFileOffsets {
                offs: self.offs.seek(SeekFrom::Current(0)).await?,
                head: self.head.seek(SeekFrom::Current(0)).await?,
                data: self.data.seek(SeekFrom::Current(0)).await?,
            }
        )
    }

    pub async fn truncate(&mut self) -> Result<()> {
        self.reset().await?;
        self.offs.set_len(0).await?;
        self.head.set_len(0).await?;
        self.data.set_len(0).await?;
        Ok(())
    }

    pub async fn reset(&mut self) -> Result<()> {
        self.offs.seek(SeekFrom::Start(0)).await?;
        self.head.seek(SeekFrom::Start(0)).await?;
        self.data.seek(SeekFrom::Start(0)).await?;
        Ok(())
    }

    async fn reset_at(&mut self, pos: &SplitLogFileOffsets, err: Option<Error>) -> Result<()> {
        self.offs.seek(SeekFrom::Start(pos.offs)).await?;
        self.head.seek(SeekFrom::Start(pos.head)).await?;
        self.data.seek(SeekFrom::Start(pos.data)).await?;
        match err {
            Some(a) => Result::Err(a),
            None => Ok(()),
        }
    }

    pub async fn write(&mut self, header: &[u8], data: &[u8]) -> Result<u32>
    {
        let restore = self.offsets().await?;

        match self.head.write(header).await {
            Err(a) => { self.reset_at(&restore, Some(a)).await?; },
            _ => {}
        }
        
        match self.data.write(data).await {
            Err(a) => { self.reset_at(&restore, Some(a)).await?; },
            _ => {}
        }

        match self.offs.write_u64(restore.head + header.len() as u64).await {
            Err(a) => { self.reset_at(&restore, Some(a)).await?; },
            _ => {}
        }

        match self.offs.write_u64(restore.data + data.len() as u64).await {
            Err(a) => { self.reset_at(&restore, Some(a)).await?; },
            _ => {}
        }
        
        Ok(0)
    }

    pub async fn read(&mut self) -> Option<(Vec<u8>,Vec<u8>)> {
        let cur_head = self.head.seek(SeekFrom::Current(0)).await.unwrap();
        let cur_data = self.data.seek(SeekFrom::Current(0)).await.unwrap();

        let next_head = self.offs.read_u64().await.unwrap_or_default();
        let next_data = self.offs.read_u64().await.unwrap_or_default();
        
        let stride_head = next_head - cur_head;
        let stride_data = next_data - cur_data;
        if stride_head <= 0 || stride_data <= 0 {
            return None;
        }

        let mut buff_head: Vec<u8> = Vec::with_capacity(stride_head as usize);
        let mut buff_data: Vec<u8> = Vec::with_capacity(stride_data as usize);

        match self.head.read(buff_head.as_mut_slice()).await {
            Ok(a) if a == stride_head as usize => { },
            _ => return None,
        }
        match self.data.read(buff_data.as_mut_slice()).await {
            Ok(a) if a == stride_data as usize => { },
            _ => return None,
        }

        Some(
            (buff_head, buff_data)
        )
    }
}
#[allow(dead_code)]
pub enum LoggingMode {
    FrontOnly,
    BothBuffers
}

pub struct RedoLogProtected {
    front: SplitLogFile,
    back: SplitLogFile,
    mode: LoggingMode,
}

impl RedoLogProtected
{
    async fn truncate(&mut self) -> Result<()> {
        self.front.truncate().await?;
        self.back.truncate().await?;
        Ok(())
    }

    async fn reset(&mut self) -> Result<()> {
        self.front.reset().await?;
        self.back.reset().await?;
        Ok(())
    }

    async fn write(&mut self, header: &[u8], data: &[u8]) -> Result<u32> {
        match self.mode {
            LoggingMode::FrontOnly => {
                let ret = self.front.write(header, data).await?;
                Ok(ret)
            }
            LoggingMode::BothBuffers => {
                let ret = self.front.write(header, data).await?;
                self.back.write(header, data).await?;
                Ok(ret)
            }
        }
    }

    async fn read(&mut self) -> Option<(Vec<u8>,Vec<u8>)> {
        self.front.read().await
    }
}

pub struct RedoLog {
    inside: Arc<Mutex<RedoLogProtected>>,
}

impl RedoLog
{
    #[allow(dead_code)]
    pub async fn new(cfg: &impl ConfigStorage, key: &impl ChainKey) -> RedoLog {
        RedoLog {
            inside: Arc::new(Mutex::new(
                RedoLogProtected {
                    front: SplitLogFile::new(cfg, key).await,
                    back: SplitLogFile::new(cfg, key).await,
                    mode: LoggingMode::FrontOnly,
                }
            ))
        }
    }

    pub async fn truncate(&mut self) -> Result<()> {
        let mut lock = self.inside.lock().await;
        lock.truncate().await
    }

    pub async fn reset(&mut self) -> Result<()> {
        let mut lock = self.inside.lock().await;
        lock.reset().await
    }

    pub async fn write(&mut self, header: &[u8], data: &[u8]) -> Result<u32> {
        let mut lock = self.inside.lock().await;
        lock.write(header, data).await
    }

    pub async fn read(&mut self) -> Option<(Vec<u8>,Vec<u8>)> {
        let mut lock = self.inside.lock().await;
        lock.read().await
    }
}

#[test]
fn test_redo_log() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {    
        let mut rl = RedoLog::new(&mock_test_config(), &mock_test_chain_key()).await;
        rl.truncate().await;

        let mock_head = b"header";
        let mock_data = b"data";

        rl.write(mock_head, mock_data).await.expect("Failed to write the object");
        let (test_head, test_data) = rl.read().await.expect("Failed to read the object data");

        assert_eq!(mock_head.to_vec(), test_head);
        assert_eq!(mock_data.to_vec(), test_data);
    });
}