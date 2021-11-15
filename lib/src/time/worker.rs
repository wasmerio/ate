use fxhash::FxHashMap;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

use crate::conf::*;
use crate::engine::TaskEngine;
use crate::error::*;

use once_cell::sync::Lazy;
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use tokio::sync::watch::*;
use tokio::sync::Mutex;

use super::ntp::NtpResult;

#[derive(Debug)]
pub struct NtpWorker {
    result: Receiver<NtpResult>,
}

pub struct NtpOffset {
    pub offset_ms: i64,
    pub accurate: bool,
}

pub struct NtpPing {
    pub roundtrip_ms: u64,
    pub accurate: bool,
}

pub struct NtpTimestamp {
    pub since_the_epoch: Duration,
    pub accurate: bool,
}

static TIMESTAMP_WORKER: Lazy<Mutex<FxHashMap<String, Arc<NtpWorker>>>> =
    Lazy::new(|| Mutex::new(FxHashMap::default()));

impl NtpWorker {
    async fn new(pool: String, port: u16, tolerance_ms: u32) -> Result<Arc<NtpWorker>, TimeError> {
        debug!("ntp service started for {}@{}", pool, port);
        let tolerance_ms_loop = tolerance_ms;

        // Make an inaccure NTP result using the system clock
        let start = SystemTime::now();
        let since_the_epoch = start.duration_since(UNIX_EPOCH)?.as_nanos();
        let ntp_result = NtpResult {
            sec: (since_the_epoch / 1000000000u128) as u32,
            nsec: (since_the_epoch % 1000000000u128) as u32,
            roundtrip: u64::MAX,
            offset: 0i64,
            accurate: false,
        };

        let (tx, rx) = channel(ntp_result);
        let ret = Arc::new(NtpWorker { result: rx });

        let bt_pool = pool.clone();
        TaskEngine::spawn(async move {
            let mut backoff_time = 50;
            let mut best_ping = u32::MAX;
            loop {
                match super::ntp::query_ntp_retry(&bt_pool, port, tolerance_ms_loop, 10).await {
                    Ok(r) => {
                        let ping = Duration::from_micros(r.roundtrip()).as_millis() as u32;
                        if best_ping == u32::MAX || ping < best_ping + 50 {
                            best_ping = ping;
                            let res = tx.send(r);
                            if let Err(err) = res {
                                warn!("{}", err);
                                break;
                            }
                        }
                        crate::engine::sleep(Duration::from_secs(20)).await;
                        backoff_time = 50;
                    }
                    _ => {
                        crate::engine::sleep(Duration::from_millis(backoff_time)).await;
                        backoff_time = (backoff_time * 120) / 100;
                        backoff_time = backoff_time + 50;
                        if backoff_time > 10000 {
                            backoff_time = 10000;
                        }
                    }
                }
            }
        });

        debug!("ntp service ready for {}@{}", pool, port);
        Ok(ret)
    }

    pub async fn create(cfg: &ConfAte, tolerance_ms: u32) -> Result<Arc<NtpWorker>, TimeError> {
        let pool = cfg.ntp_pool.clone();
        let port = cfg.ntp_port;
        let ntp_worker = {
            let key = format!("{}:{}", cfg.ntp_pool, cfg.ntp_port);
            let mut guard = TIMESTAMP_WORKER.lock().await;
            match guard.get(&key) {
                Some(a) => Arc::clone(a),
                None => {
                    let worker = NtpWorker::new(pool, port, tolerance_ms).await?;
                    guard.insert(key, Arc::clone(&worker));
                    worker
                }
            }
        };
        Ok(ntp_worker)
    }

    #[allow(dead_code)]
    fn current_offset_ms(&self) -> NtpOffset {
        let guard = self.result.borrow();
        let ret = guard.offset() / 1000;
        NtpOffset {
            offset_ms: ret,
            accurate: guard.accurate,
        }
    }

    #[allow(dead_code)]
    fn current_ping_ms(&self) -> NtpPing {
        let guard = self.result.borrow();
        let ret = guard.roundtrip() / 1000;
        NtpPing {
            roundtrip_ms: ret,
            accurate: guard.accurate,
        }
    }

    pub async fn wait_for_high_accuracy(&self) {
        let mut result = self.result.clone();
        while result.borrow().accurate == false {
            if let Err(err) = result.changed().await {
                error!("{}", err);
                break;
            }
        }
    }

    pub fn is_accurate(&self) -> bool {
        self.result.borrow().accurate
    }

    pub fn current_timestamp(&self) -> Result<NtpTimestamp, TimeError> {
        let start = SystemTime::now();
        let mut since_the_epoch = start.duration_since(UNIX_EPOCH)?;

        let guard = self.result.borrow();
        let mut offset = guard.offset();
        if offset >= 0 {
            since_the_epoch = since_the_epoch + Duration::from_micros(offset as u64);
        } else {
            offset = 0 - offset;
            since_the_epoch = since_the_epoch - Duration::from_micros(offset as u64);
        }

        Ok(NtpTimestamp {
            since_the_epoch,
            accurate: guard.accurate,
        })
    }
}
