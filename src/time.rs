use super::error::*;
use super::meta::*;
use super::lint::*;
use super::plugin::*;
use super::index::*;
use super::session::*;
use super::sink::*;
use super::transform::*;
use super::compact::*;
use super::validator::*;
use super::crypto::Hash;
use super::conf::*;

use std::{ops::Deref, sync::Arc};
use std::sync::Mutex;
use std::sync::RwLock;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

mod ntp;

use ntp::NtpResult;

pub struct TimestampEnforcer {
    pub cursor: Duration,
    pub tolerance: Duration,
    pub ntp_pool: String,
    pub ntp_port: u32,
    pub ntp_result: Arc<RwLock<NtpResult>>,
    pub bt_exit: Arc<Mutex<bool>>,
}

impl Drop
for TimestampEnforcer
{
    fn drop(&mut self) {
        *self.bt_exit.lock().unwrap() = true;
    }
}

impl TimestampEnforcer
{
    #[allow(dead_code)]
    pub fn new(cfg: &impl ConfigNtp, tolerance_ms: u32) -> Result<TimestampEnforcer, TimeError>
    {
        let tolerance_ms_loop = tolerance_ms;
        let tolerance_ms_seed = tolerance_ms * 3;

        let pool = Arc::new(cfg.ntp_pool());
        let ntp_result = Arc::new(RwLock::new(ntp::query_ntp_retry(pool.deref(), cfg.ntp_port(), tolerance_ms_seed, 10)?));
        let bt_exit = Arc::new(Mutex::new(false));

        let bt_best_ping = Duration::from_micros(ntp_result.write().unwrap().roundtrip()).as_millis() as u32;
        let bt_pool = Arc::new(cfg.ntp_pool());
        let bt_port = cfg.ntp_port();
        let bt_exit2 = bt_exit.clone();
        let bt_result = ntp_result.clone();

        std::thread::spawn(move || {
            let mut n: u32 = 0;
            let mut best_ping = bt_best_ping;

            while *bt_exit2.lock().unwrap() == false {
                if n > 200 {
                    n = 0;
                    match ntp::query_ntp_retry(bt_pool.deref(), bt_port, tolerance_ms_loop, 10) {
                        Ok(r) =>
                        {
                            let ping = Duration::from_micros(r.roundtrip()).as_millis() as u32;
                            if ping < best_ping + 50 {
                                best_ping = ping;
                                *bt_result.write().unwrap() = r;
                            }
                        },
                        _ => {}
                    }
                }
                
                std::thread::sleep(Duration::from_millis(100));
                n = n + 1;
            }
        });

        let tolerance = Duration::from_millis(tolerance_ms as u64);
        Ok(
            TimestampEnforcer
            {
                cursor: tolerance,
                tolerance: tolerance,
                ntp_pool: cfg.ntp_pool(),
                ntp_port: cfg.ntp_port(),
                ntp_result: ntp_result,
                bt_exit: bt_exit.clone(),
            }
        )
    }

    #[allow(dead_code)]
    pub fn current_offset_ms(&self) -> i64
    {
        let ret = self.ntp_result.read().unwrap().offset() / 1000;
        ret
    }

    #[allow(dead_code)]
    pub fn current_ping_ms(&self) -> u64
    {
        let ret = self.ntp_result.read().unwrap().roundtrip() / 1000;
        ret
    }

    pub fn current_timestamp(&self) -> Result<Duration, TimeError>
    {
        let start = SystemTime::now();
        let mut since_the_epoch = start
            .duration_since(UNIX_EPOCH)?;

        let mut offset = self.ntp_result.read().unwrap().offset();
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

    fn get_timestamp(meta: &Metadata) -> Option<&MetaTimestamp> {
        meta.core
            .iter()
            .filter_map(|m| {
                match m {
                    CoreMetadata::Timestamp(time) => Some(time),
                    _ => None,
                }
            })
            .next()
    }
}

impl Default
for TimestampEnforcer
{
    fn default() -> TimestampEnforcer
    {
        let cfg = DiscreteConfig::default();
        TimestampEnforcer::new(&cfg, 200).unwrap()
    }
}

impl EventMetadataLinter
for TimestampEnforcer
{
    fn metadata_lint_event(&self, _meta: &Metadata, _session: &Session)-> Result<Vec<CoreMetadata>, LintError> {
        let mut ret = Vec::new();

        //println!("TIME: {} with offset of {} and ping of {}", self.current_timestamp()?, self.current_offset_ms(), self.current_ping_ms());

        ret.push(CoreMetadata::Timestamp(
            MetaTimestamp {
                time_since_epoch_ms: self.current_timestamp()?.as_millis() as u64,
            }
        ));

        Ok(ret)
    }
}

impl EventSink
for TimestampEnforcer
{
    fn feed(&mut self, meta: &Metadata, _data_hash: &Option<Hash>) -> Result<(), SinkError>
    {
        if let Some(time) = TimestampEnforcer::get_timestamp(meta) {
            let time = Duration::from_millis(time.time_since_epoch_ms);
            if time > self.cursor {
                self.cursor = time;
            }
        }
        Ok(())
    }   
}

impl EventIndexer
for TimestampEnforcer
{
}

impl EventDataTransformer
for TimestampEnforcer
{
}

impl EventCompactor
for TimestampEnforcer
{
}

impl EventValidator
for TimestampEnforcer
{
    fn validate(&self, validation_data: &ValidationData) -> Result<ValidationResult, ValidationError>
    {
        // If it does not have a timestamp then we can not accept it
        let time = match TimestampEnforcer::get_timestamp(&validation_data.meta) {
            Some(m) => m,
            None => {
                return match validation_data.meta.needs_signature() {
                    true => Err(ValidationError::Time(TimeError::NoTimestamp)),
                    false => Ok(ValidationResult::Abstain)
                };
            },
        };

        // Check its within the time range
        let timestamp = Duration::from_millis(time.time_since_epoch_ms);
        let min_timestamp = self.cursor - self.tolerance;
        let max_timestamp = self.current_timestamp()? + self.tolerance;
        
        if timestamp < min_timestamp ||
           timestamp > max_timestamp
        {
            return Err(ValidationError::Time(TimeError::OutOfBounds(timestamp)));
        }

        // All good
        Ok(ValidationResult::Abstain)
    }
}

impl EventPlugin
for TimestampEnforcer
{
}