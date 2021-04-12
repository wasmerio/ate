#![allow(unused_imports)]
use fxhash::FxHashMap;
use log::{error, info, debug};

use super::error::*;
use super::meta::*;
use super::lint::*;
use super::plugin::*;
use super::index::*;
use super::session::*;
use super::sink::*;
use super::transform::*;
use super::validator::*;
use super::conf::*;
use super::transaction::*;
use super::event::EventHeader;

use std::{ops::Deref, sync::Arc};
use parking_lot::Mutex as PMutex;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use std::sync::Once;
use once_cell::sync::Lazy;

mod ntp;

use ntp::NtpResult;

#[derive(Debug, Clone)]
pub struct TimestampEnforcer {
    pub cursor: Duration,
    pub tolerance: Duration,
    pub ntp_pool: String,
    pub ntp_port: u16,
    pub(crate) ntp_worker: Arc<NtpWorker>,
}

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
        let ntp_result = ntp::query_ntp_with_backoff(pool.deref(), port, tolerance_ms_seed, 10).await;
        
        let bt_best_ping = Duration::from_micros(ntp_result.roundtrip()).as_millis() as u32;
        let bt_pool = Arc::new(pool.clone());
        
        let ret = Arc::new(NtpWorker {
            result: PMutex::new(ntp_result)
        });

        let worker_ret = Arc::clone(&ret);
        tokio::spawn(async move {
            let mut best_ping = bt_best_ping;
            loop {
                match ntp::query_ntp_retry(bt_pool.deref(), port, tolerance_ms_loop, 10).await {
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

    pub fn current_offset_ms(&self) -> i64
    {
        let ret = self.result.lock().offset() / 1000;
        ret
    }

    pub fn current_ping_ms(&self) -> u64
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

impl TimestampEnforcer
{
    #[allow(dead_code)]
    pub async fn new(cfg: &ConfAte, tolerance_ms: u32) -> Result<TimestampEnforcer, TimeError>
    {
        let tolerance = Duration::from_millis(tolerance_ms as u64);
        Ok(
            TimestampEnforcer
            {
                cursor: tolerance,
                tolerance: tolerance,
                ntp_pool: cfg.ntp_pool.clone(),
                ntp_port: cfg.ntp_port,
                ntp_worker: NtpWorker::create(cfg, tolerance_ms).await?,
            }
        )
    }
}

impl EventMetadataLinter
for TimestampEnforcer
{
    fn clone_linter(&self) -> Box<dyn EventMetadataLinter> {
        Box::new(self.clone())
    }

    fn metadata_lint_event(&self, _meta: &Metadata, _session: &Session, _trans_meta: &TransactionMetadata)-> Result<Vec<CoreMetadata>, LintError> {
        let mut ret = Vec::new();

        //println!("TIME: {} with offset of {} and ping of {}", self.current_timestamp()?, self.current_offset_ms(), self.current_ping_ms());

        ret.push(CoreMetadata::Timestamp(
            MetaTimestamp {
                time_since_epoch_ms: self.ntp_worker.current_timestamp()?.as_millis() as u64,
            }
        ));

        Ok(ret)
    }
}

impl EventSink
for TimestampEnforcer
{
    fn feed(&mut self, header: &EventHeader, _conversation: Option<&Arc<ConversationSession>>) -> Result<(), SinkError>
    {
        if let Some(time) = header.meta.get_timestamp() {
            let time = Duration::from_millis(time.time_since_epoch_ms);
            if time > self.cursor {
                self.cursor = time;
            }
        }
        Ok(())
    }   

    fn reset(&mut self) {
        self.cursor = self.tolerance.clone();
    }
}

impl EventIndexer
for TimestampEnforcer
{
    fn clone_indexer(&self) -> Box<dyn EventIndexer> {
        Box::new(self.clone())
    }
}

impl EventDataTransformer
for TimestampEnforcer
{
    fn clone_transformer(&self) -> Box<dyn EventDataTransformer> {
        Box::new(self.clone())
    }
}

impl EventValidator
for TimestampEnforcer
{
    fn clone_validator(&self) -> Box<dyn EventValidator> {
        Box::new(self.clone())
    }

    fn validate(&self, header: &EventHeader, _conversation: Option<&Arc<ConversationSession>>) -> Result<ValidationResult, ValidationError>
    {
        // If it does not have a timestamp then we can not accept it
        let time = match header.meta.get_timestamp() {
            Some(m) => m,
            None => {
                return match header.meta.needs_signature() {
                    true => {
                        debug!("rejected event due to missing timestamp");
                        Err(ValidationError::Trust(TrustError::Time(TimeError::NoTimestamp)))
                    },
                    false => Ok(ValidationResult::Abstain)
                };
            },
        };

        // Check its within the time range
        let timestamp = Duration::from_millis(time.time_since_epoch_ms);
        let min_timestamp = self.cursor - self.tolerance;
        let max_timestamp = self.ntp_worker.current_timestamp()? + self.tolerance;
        
        if timestamp < min_timestamp ||
           timestamp > max_timestamp
        {
            debug!("rejected event due to out-of-bounds timestamp ({:?} vs {:?})", self.cursor, timestamp);
            return Err(ValidationError::Trust(TrustError::Time(TimeError::OutOfBounds(self.cursor - timestamp))));
        }

        // All good
        Ok(ValidationResult::Abstain)
    }
}

impl EventPlugin
for TimestampEnforcer
{
    fn clone_plugin(&self) -> Box<dyn EventPlugin> {
        Box::new(self.clone())
    }
}