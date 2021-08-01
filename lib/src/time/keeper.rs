#[allow(unused_imports)]
use tracing::{error, info, debug};

use crate::error::*;
use crate::conf::*;

#[cfg(feature = "enable_ntp")]
use std::{sync::Arc};
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
#[cfg(feature = "enable_ntp")]
use super::worker::NtpWorker;
use super::ChainTimestamp;

#[derive(Debug, Clone)]
pub struct TimeKeeper
{    
    pub tolerance: Duration,
    #[cfg(feature = "enable_ntp")]
    pub ntp_pool: String,
    #[cfg(feature = "enable_ntp")]
    pub ntp_port: u16,
    #[cfg(feature = "enable_ntp")]
    pub(crate) ntp_worker: Option<Arc<NtpWorker>>,
}

impl TimeKeeper
{
    #[allow(unused_variables)]
    #[allow(dead_code)]
    pub async fn new(cfg: &ConfAte, tolerance_ms: u32) -> Result<TimeKeeper, TimeError>
    {
        let tolerance = Duration::from_millis(tolerance_ms as u64);
        Ok(
            TimeKeeper
            {
                tolerance: tolerance,
                #[cfg(feature = "enable_ntp")]
                ntp_pool: cfg.ntp_pool.clone(),
                #[cfg(feature = "enable_ntp")]
                ntp_port: cfg.ntp_port,
                #[cfg(feature = "enable_ntp")]
                ntp_worker: match cfg.ntp_sync {
                    true => Some(NtpWorker::create(cfg, tolerance_ms).await?),
                    false => None,
                },
            }
        )
    }

    pub fn current_timestamp_as_duration(&self) -> Result<Duration, TimeError> {
        #[cfg(not(feature = "enable_ntp"))]
        {
            let start = SystemTime::now();
            let since_the_epoch = start
                .duration_since(UNIX_EPOCH)?;
            Ok(since_the_epoch)
        }
        #[cfg(feature = "enable_ntp")]
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
        Ok(ChainTimestamp::from(self.current_timestamp_as_duration()?.as_millis() as u64))
    }
}