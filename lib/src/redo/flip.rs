#[allow(unused_imports)]
use log::{error, info, warn, debug};

use async_trait::async_trait;
use tokio::io::Result;

use crate::{crypto::AteHash};
use crate::event::*;
use crate::error::*;

use super::file::LogFile;
use super::api::LogWritable;
use super::core::RedoLog;

pub struct FlippedLogFile {
    pub(super) log_file: LogFile,
    pub(crate) event_summary: Vec<EventHeaderRaw>,
}

#[async_trait]
impl LogWritable
for FlippedLogFile
{
    #[allow(dead_code)]
    async fn write(&mut self, evt: &EventData) -> std::result::Result<u64, SerializationError> {
        let ret = self.log_file.write(evt).await?;
        self.event_summary.push(evt.as_header_raw()?);
        Ok(ret)
    }

    async fn flush(&mut self) -> Result<()> {
        self.log_file.flush().await
    }
}

impl FlippedLogFile
{
    pub(super) async fn copy_log_file(&mut self) -> Result<LogFile> {
        let new_log_file = self.log_file.copy().await?;
        Ok(new_log_file)
    }

    #[allow(dead_code)]
    pub(super) fn count(&self) -> usize {
        self.log_file.count()
    }

    pub(super) fn drain_events(&mut self) -> Vec<EventHeaderRaw>
    {
        let mut ret = Vec::new();
        for evt in self.event_summary.drain(..) {
            ret.push(evt);
        }
        ret
    }

    #[allow(dead_code)]
    pub(crate) async fn copy_event(&mut self, from_log: &RedoLog, from_pointer: AteHash) -> std::result::Result<u64, LoadError> {
        Ok(self.log_file.copy_event(&from_log.log_file, from_pointer).await?)
    }
}

pub(super) struct RedoLogFlip {
    pub deferred: Vec<EventData>,
}