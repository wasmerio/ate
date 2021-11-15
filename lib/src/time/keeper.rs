#[allow(unused_imports)]
use tracing::{debug, error, info};

use crate::conf::*;
use crate::error::*;

#[cfg(feature = "enable_ntp")]
use super::worker::NtpWorker;
use super::ChainTimestamp;
#[cfg(feature = "enable_ntp")]
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone)]
pub struct TimeKeeper {
    pub tolerance: Duration,
    #[cfg(feature = "enable_ntp")]
    pub ntp_pool: String,
    #[cfg(feature = "enable_ntp")]
    pub ntp_port: u16,
    #[cfg(feature = "enable_ntp")]
    pub(crate) ntp_worker: Option<Arc<NtpWorker>>,
}

impl TimeKeeper {
    #[allow(unused_variables)]
    #[allow(dead_code)]
    pub async fn new(cfg: &ConfAte, tolerance_ms: u32) -> Result<TimeKeeper, TimeError> {
        let tolerance = Duration::from_millis(tolerance_ms as u64);
        Ok(TimeKeeper {
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
        })
    }

    pub async fn wait_for_high_accuracy(&self) {
        #[cfg(feature = "enable_ntp")]
        if let Some(worker) = &self.ntp_worker {
            worker.wait_for_high_accuracy().await;
        }
    }

    pub fn has_converged(&self) -> bool {
        #[cfg(feature = "enable_ntp")]
        if let Some(worker) = &self.ntp_worker {
            return worker.is_accurate();
        }
        true
    }

    pub fn current_timestamp_as_duration(&self) -> Result<Duration, TimeError> {
        #[cfg(not(feature = "enable_ntp"))]
        {
            let start = SystemTime::now();
            let since_the_epoch = start.duration_since(UNIX_EPOCH)?;
            Ok(since_the_epoch)
        }
        #[cfg(feature = "enable_ntp")]
        Ok(match &self.ntp_worker {
            Some(worker) => worker.current_timestamp()?.since_the_epoch,
            None => {
                let start = SystemTime::now();
                let since_the_epoch = start.duration_since(UNIX_EPOCH)?;
                since_the_epoch
            }
        })
    }

    pub fn current_timestamp(&self) -> Result<ChainTimestamp, TimeError> {
        Ok(ChainTimestamp::from(
            self.current_timestamp_as_duration()?.as_millis() as u64,
        ))
    }
}
