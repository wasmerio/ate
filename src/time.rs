use super::error::*;
use super::meta::*;
use super::lint::*;
use super::session::*;
use super::crypto::Hash;

use std::{ops::Deref, sync::Arc};
use std::sync::Mutex;
use std::sync::RwLock;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

mod ntp;

use ntp::NtpResult;

pub struct EventTimestampLinter {
    pub ntp_pool: Arc<String>,
    pub ntp_port: u32,
    pub ntp_result: Arc<RwLock<NtpResult>>,
    pub bt_exit: Arc<Mutex<bool>>,
}

impl Drop
for EventTimestampLinter
{
    fn drop(&mut self) {
        *self.bt_exit.lock().unwrap() = true;
    }
}

impl EventTimestampLinter
{
    #[allow(dead_code)]
    pub fn new(pool: String, port: u32, tolerance_ms_seed: u32, tolerance_ms_loop: u32) -> Result<EventTimestampLinter, TimeError>
    {
        let pool = Arc::new(pool);
        let ntp_result = Arc::new(RwLock::new(EventTimestampLinter::query_ntp_retry(pool.deref(), port, tolerance_ms_seed, 10)?));
        let bt_exit = Arc::new(Mutex::new(false));

        let bt_best_ping = Duration::from_micros(ntp_result.write().unwrap().roundtrip()).as_millis() as u32;
        let bt_pool = pool.clone();
        let bt_port = port.clone();
        let bt_exit2 = bt_exit.clone();
        let bt_result = ntp_result.clone();

        std::thread::spawn(move || {
            let mut n: u32 = 0;
            let mut best_ping = bt_best_ping;

            while *bt_exit2.lock().unwrap() == false {
                if n > 200 {
                    n = 0;
                    match EventTimestampLinter::query_ntp_retry(bt_pool.deref(), bt_port, tolerance_ms_loop, 10) {
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

        Ok(
            EventTimestampLinter
            {
                ntp_pool: pool,
                ntp_port: port,
                ntp_result: ntp_result,
                bt_exit: bt_exit.clone(),
            }
        )
    }

    fn query_ntp(pool: &String, port: u32, tolerance_ms: u32) -> Result<NtpResult, TimeError>
    {
        let timeout =  Duration::from_millis(tolerance_ms as u64) + Duration::from_millis(50);
        let ret = ntp::request(pool.as_str(), port, timeout)?;
        let ping = Duration::from_micros(ret.roundtrip()).as_millis() as u32;
        if ping > tolerance_ms {
            return Err(TimeError::BeyondTolerance(ping as u32));
        }
        Ok(ret)
    }

    fn query_ntp_retry(pool: &String, port: u32, tolerance_ms: u32, samples: u32) -> Result<NtpResult, TimeError>
    {
        let mut best: Option<NtpResult> = None;
        let mut positives = 0;
        let mut wait_time = 50;

        for _ in 0..samples
        {
            let timeout = match &best {
                Some(b) => Duration::from_micros(b.roundtrip()) + Duration::from_millis(50),
                None => Duration::from_millis(tolerance_ms as u64),
            };

            if let Ok(ret) = ntp::request(pool.as_str(), port, timeout) {
                let current_ping = match &best {
                    Some(b) => b.roundtrip(),
                    None => u64::max_value(),
                };
                if ret.roundtrip() < current_ping {
                    best = Some(ret);
                }
                positives = positives + 1;
                if positives >= samples {
                    break;
                }
            }
            else
            {
                std::thread::sleep(Duration::from_millis(wait_time));
                wait_time = (wait_time * 120) / 100;
                 wait_time = wait_time + 50;
            }
        }

        if let Some(ret) = best {
            let ping = Duration::from_micros(ret.roundtrip()).as_millis() as u32;
            if ping <= tolerance_ms {
                return Ok(ret);
            }
        }

        EventTimestampLinter::query_ntp(pool, port, tolerance_ms)
    }

    pub fn current_offset_ms(&self) -> i64
    {
        let ret = self.ntp_result.read().unwrap().offset() / 1000;
        ret
    }

    pub fn current_ping_ms(&self) -> u64
    {
        let ret = self.ntp_result.read().unwrap().roundtrip() / 1000;
        ret
    }

    pub fn current_timestamp(&self) -> Result<u128, TimeError>
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
            since_the_epoch.as_nanos()
        )
    }
}

impl Default
for EventTimestampLinter
{
    fn default() -> EventTimestampLinter {
        EventTimestampLinter::new("pool.ntp.org".to_string(), 123, 600, 200).unwrap()
    }
}

impl<M> EventMetadataLinter<M>
for EventTimestampLinter
where M: OtherMetadata,
{
    fn metadata_lint_event(&self, _data_hash: &Option<Hash>, _meta: &MetadataExt<M>, _session: &Session)-> Result<Vec<CoreMetadata>, LintError> {
        let mut ret = Vec::new();

        //println!("TIME: {} with offset of {} and ping of {}", self.current_timestamp()?, self.current_offset_ms(), self.current_ping_ms());

        ret.push(CoreMetadata::Timestamp(
            MetaTimestamp {
                time_since_epoch_ns: self.current_timestamp()? as u64,
            }
        ));

        Ok(ret)
    }
}