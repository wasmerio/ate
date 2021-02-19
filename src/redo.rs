extern crate tokio;
extern crate bincode;

use super::conf::*;
use super::chain::*;
use super::header::*;

use std::io::SeekFrom;
use std::sync::Arc;
use tokio::{fs::File, io::{AsyncReadExt, AsyncWriteExt, AsyncSeekExt}};
use tokio::sync::Mutex;
use tokio::io::Result;
use tokio::io::Error;

#[cfg(test)]
use tokio::runtime::Runtime;

pub struct SplitLogFile
{
    pub head: File,
    pub data: File,
    pub entries: Vec<Header>,
}

impl SplitLogFile {
    async fn new(cfg: &impl ConfigStorage, key: &impl ChainKey) -> SplitLogFile {
        let path_head = format!("{}/{}.head", cfg.log_path(), key.to_key_str());
        let path_data = format!("{}/{}.data", cfg.log_path(), key.to_key_str());

        let mut ret = SplitLogFile {
            head: tokio::fs::File::create(path_head).await.unwrap(),
            data: tokio::fs::File::create(path_data).await.unwrap(),
            entries: Vec::new()
        };

        while let Some(next) = ret.read_once().await {
            ret.entries.push(next);
        }
        
        ret
    }

    #[allow(dead_code)]
    pub async fn read_all_entries(&mut self, ret: &mut Vec<Header>) -> Result<u64> {
        let mut cnt = 0;
        while let Some(next) = self.read_once().await {
            ret.push(next);
            cnt = cnt + 1;
        }
        Ok(cnt)
    }

    pub async fn read_once(&mut self) -> Option<Header> {
        let size_head = self.head.read_u64().await.ok()?;
        let mut buff_head = Vec::with_capacity(size_head as usize);
        self.head.read(buff_head.as_mut_slice()).await.ok()?;

        let ret = bincode::deserialize(buff_head.as_slice()).ok()?;
        Some(ret)
    }

    pub async fn truncate(&mut self) -> Result<()> {
        self.reset().await?;
        self.head.set_len(0).await?;
        self.data.set_len(0).await?;
        Ok(())
    }

    pub async fn reset(&mut self) -> Result<()> {
        self.head.seek(SeekFrom::Start(0)).await?;
        self.data.seek(SeekFrom::Start(0)).await?;
        Ok(())
    }

    async fn reset_at(&mut self, pos_head: u64, pos_data: u64, err: Option<Error>) -> Result<()> {
        self.head.seek(SeekFrom::Start(pos_head)).await?;
        self.data.seek(SeekFrom::Start(pos_data)).await?;
        match err {
            Some(a) => Result::Err(a),
            None => Ok(()),
        }
    }

    pub async fn write(&mut self, header: &mut Header, data: &[u8], digest: &[u8]) -> Result<()>
    {
        let restore_head = self.head.seek(SeekFrom::Current(0)).await.unwrap();
        let restore_data = self.data.seek(SeekFrom::Current(0)).await.unwrap();

        header.size_data = data.len() as u64;
        header.off_data = restore_data;
        header.size_digest = digest.len() as u64;
        header.off_digest = restore_data + data.len() as u64;

        let buff_header = bincode::serialize(&header).unwrap();

        match self.data.write(data).await {
            Err(a) => { return self.reset_at(restore_head, restore_data, Some(a)).await; },
            _ => {}
        }

        match self.data.write(digest).await {
            Err(a) => { return self.reset_at(restore_head, restore_data, Some(a)).await; },
            _ => {}
        }

        match self.head.write_u64(buff_header.len() as u64).await {
            Err(a) => { return self.reset_at(restore_head, restore_data, Some(a)).await; },
            _ => {}
        }
        
        match self.head.write(buff_header.as_slice()).await {
            Err(a) => { return self.reset_at(restore_head, restore_data, Some(a)).await; },
            _ => {}
        }
        
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn read(&mut self) -> Option<Header> {
        let size_head = self.head.read_u64().await.unwrap_or_default();
        let mut buff_head: Vec<u8> = Vec::with_capacity(size_head as usize);
        
        match self.head.read(buff_head.as_mut_slice()).await {
            Ok(a) if a == size_head as usize => { },
            _ => return None,
        }

        return bincode::deserialize(buff_head.as_slice()).ok();
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

    async fn write(&mut self, header: &mut Header, data: &[u8], digest: &[u8]) -> Result<()> {
        match self.mode {
            LoggingMode::FrontOnly => {
                self.front.write(header, data, digest).await
            }
            LoggingMode::BothBuffers => {
                let _ = self.front.write(header, data, digest).await?;
                self.back.write(header, data, digest).await
            }
        }
    }

    async fn read(&mut self) -> Option<Header> {
        self.front.read().await
    }
}

pub struct RedoLog {
    #[allow(unused_variables)]
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

    #[allow(dead_code)]
    pub async fn truncate(&mut self) -> Result<()> {
        let mut lock = self.inside.lock().await;
        lock.truncate().await
    }

    #[allow(dead_code)]
    pub async fn reset(&mut self) -> Result<()> {
        let mut lock = self.inside.lock().await;
        lock.reset().await
    }

    #[allow(dead_code)]
    pub async fn write(&mut self, header: &mut Header, data: &[u8], digest: &[u8]) -> Result<()> {
        let mut lock = self.inside.lock().await;
        lock.write(header, data, digest).await
    }

    #[allow(dead_code)]
    pub async fn read(&mut self) -> Option<Header> {
        let mut lock = self.inside.lock().await;
        lock.read().await
    }
}

#[test]
fn test_redo_log() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {    
        let mut rl = RedoLog::new(&mock_test_config(), &mock_test_chain_key()).await;
        rl.truncate().await.expect("Failed to truncate the redo log");

        let mut mock_head = Header::default();
        let mock_digest = Vec::with_capacity(100);
        let mock_data = Vec::with_capacity(1000);

        rl.write(&mut mock_head, mock_data.as_slice(), mock_digest.as_slice()).await.expect("Failed to write the object");
    });
}