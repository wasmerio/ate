#[allow(unused_imports)]
use log::{error, info, debug};
use fxhash::FxHashMap;

use crate::error::*;
use crate::conf::*;

use std::{ops::Deref, sync::Arc};
use parking_lot::Mutex as PMutex;
use tokio::sync::Mutex;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use once_cell::sync::Lazy;

use super::ntp::NtpResult;

#[derive(Debug)]
pub struct NtpWorker
{
    result: PMutex<NtpResult>
}

static TIMESTAMP_WORKER: Lazy<Mutex<FxHashMap<String, Arc<NtpWorker>>>> = Lazy::new(|| Mutex::new(FxHashMap::default()));

impl NtpWorker
{
    async fn new(pool: String, port: u16, tolerance_ms: u32) -> Result<Arc<NtpWorker>, TimeError>
    {
        debug!("ntp service started for {}@{}", pool, port);
        let tolerance_ms_loop = tolerance_ms;
        let tolerance_ms_seed = tolerance_ms * 3;

        let pool = Arc::new(pool.clone());
        let ntp_result = super::ntp::query_ntp_with_backoff(pool.deref(), port, tolerance_ms_seed, 10).await;
        
        let bt_best_ping = Duration::from_micros(ntp_result.roundtrip()).as_millis() as u32;
        let bt_pool = Arc::new(pool.clone());
        
        let ret = Arc::new(NtpWorker {
            result: PMutex::new(ntp_result)
        });

        let worker_ret = Arc::clone(&ret);
        tokio::spawn(async move {
            let mut best_ping = bt_best_ping;
            loop {
                match super::ntp::query_ntp_retry(bt_pool.deref(), port, tolerance_ms_loop, 10).await {
                    Ok(r) =>
                    {
                        let ping = Duration::from_micros(r.roundtrip()).as_millis() as u32;
                        if ping < best_ping + 50 {
                            best_ping = ping;
                            *worker_ret.result.lock() = r;
                        }
                    },
                    _ => { }
                }
                
                tokio::time::sleep(Duration::from_secs(20)).await;
            }
        });

        debug!("ntp service ready for {}@{}", pool, port);
        Ok(ret)
    }

    pub async fn create(cfg: &ConfAte, tolerance_ms: u32) -> Result<Arc<NtpWorker>, TimeError>
    {
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
    fn current_offset_ms(&self) -> i64
    {
        let ret = self.result.lock().offset() / 1000;
        ret
    }

    #[allow(dead_code)]
    fn current_ping_ms(&self) -> u64
    {
        let ret = self.result.lock().roundtrip() / 1000;
        ret
    }

    pub fn current_timestamp(&self) -> Result<Duration, TimeError>
    {
        let start = SystemTime::now();
        let mut since_the_epoch = start
            .duration_since(UNIX_EPOCH)?;

        let mut offset = self.result.lock().offset();
        if offset >= 0 {
            since_the_epoch = since_the_epoch + Duration::from_micros(offset as u64);
        } else {
            offset = 0 - offset;
            since_the_epoch = since_the_epoch - Duration::from_micros(offset as u64);
        }

        Ok(
            since_the_epoch
        )
    }
}