#[allow(unused_imports)]
use log::{error, info, debug};

use crate::error::*;
use crate::conf::*;

use std::{sync::Arc};
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use super::worker::NtpWorker;
use super::ChainTimestamp;

#[derive(Debug, Clone)]
pub struct TimeKeeper
{    
    pub tolerance: Duration,
    pub ntp_pool: String,
    pub ntp_port: u16,
    pub(crate) ntp_worker: Option<Arc<NtpWorker>>,
}

impl TimeKeeper
{
    #[allow(dead_code)]
    pub async fn new(cfg: &ConfAte, tolerance_ms: u32) -> Result<TimeKeeper, TimeError>
    {
        let tolerance = Duration::from_millis(tolerance_ms as u64);
        Ok(
            TimeKeeper
            {
                tolerance: tolerance,
                ntp_pool: cfg.ntp_pool.clone(),
                ntp_port: cfg.ntp_port,
                ntp_worker: match cfg.ntp_sync {
                    true => Some(NtpWorker::create(cfg, tolerance_ms).await?),
                    false => None,
                },
            }
        )
    }

    pub fn current_timestamp_internal(&self) -> Result<Duration, TimeError> {
        Ok(match &self.ntp_worker {
            Some(worker) => worker.current_timestamp()?,
            None => {
                let start = SystemTime::now();
                let since_the_epoch = start
                    .duration_since(UNIX_EPOCH)?;
                since_the_epoch
            }
        })        
    }

    pub fn current_timestamp(&self) -> Result<ChainTimestamp, TimeError>
    {
        Ok(ChainTimestamp::from(self.current_timestamp_internal()?.as_millis() as u64))
    }
}