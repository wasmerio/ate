#[allow(unused_imports)]
use log::{info, warn, debug, error};
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::watch;
use parking_lot::Mutex;
use tokio::select;

use super::CompactMode;

const GROWTH_FACTOR_IGNORE_SMALLER_THAN_SIZE: u64 = 2097152;

pub(crate) struct CompactNotifications
{
    pub log_size: watch::Sender<u64>,
}

pub(crate) struct CompactState
{
    pub mode: CompactMode,
    pub log_size: watch::Receiver<u64>,
    pub last_size: Arc<Mutex<u64>>,
    pub last_compact: Arc<Mutex<Option<Instant>>>
}

impl CompactState
{
    pub fn new(mode: CompactMode, size: u64) -> (CompactNotifications, CompactState) {
        let (modified_tx, modified_rx) = watch::channel::<u64>(size);

        (
            CompactNotifications {
                log_size: modified_tx,
            },
            CompactState {
                mode,
                log_size: modified_rx,
                last_size: Arc::new(Mutex::new(size)),
                last_compact: Arc::new(Mutex::new(None)),
            },
        )
    }

    pub async fn wait_for_compact(&mut self) -> Result<(), watch::error::RecvError> {
        loop {
            let initial_size = {
                let mut guard = self.last_size.lock();
                let mut ret = *guard;
                
                // If the size has gone backwards (likely due to compaction) then move the cursor back
                let cur = *self.log_size.borrow();
                if cur < ret {
                    *guard = cur;
                    ret = cur;
                }
                
                ret
            };

            let deadtime_compact = Arc::clone(&self.last_compact);
            let deadtime = move |duration: Duration| {
                let mut guard = deadtime_compact.lock();
                match *guard {
                    Some(a) => {
                        let already = a.elapsed();
                        if already > duration {
                            Duration::from_secs(0)
                        } else {
                            duration - already
                        }
                    }
                    None => {
                        *guard = Some(Instant::now());
                        duration
                    }
                }
            };

            match self.mode {
                CompactMode::Never => {
                    tokio::time::sleep(Duration::from_secs(u64::MAX)).await;
                },
                CompactMode::Timer(duration) => {
                    let deadtime = deadtime(duration);
                    tokio::time::sleep(deadtime).await;
                    break;
                }
                CompactMode::Modified => {
                    self.log_size.changed().await?;
                    break;
                }
                CompactMode::GrowthSize(target) => {
                    let final_size = *self.log_size.borrow();
                    if final_size > initial_size && final_size - initial_size >= target {
                        break;
                    }
                    self.log_size.changed().await?;
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                CompactMode::GrowthFactor(target) => {
                    let target = 1.0f32 + target;
                    let target = (initial_size as f32 * target) as u64;
                    let cur = *self.log_size.borrow();
                    if cur >= GROWTH_FACTOR_IGNORE_SMALLER_THAN_SIZE && cur >= target {
                        break;
                    }
                    self.log_size.changed().await?;
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                CompactMode::GrowthSizeOrTimer { growth, timer } => {
                    let final_size = *self.log_size.borrow();
                    if final_size > initial_size && final_size - initial_size >= growth {
                        break;
                    }
                    let deadtime = deadtime(timer);
                    select! {
                        a = self.log_size.changed() => {
                            a?;
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        },
                        () = tokio::time::sleep(deadtime) => { break; },
                    };
                }
                CompactMode::GrowthFactorOrTimer { growth, timer } => {
                    let target = 1.0f32 + growth;
                    let target = (initial_size as f32 * target) as u64;
                    let cur = *self.log_size.borrow();
                    if cur >= GROWTH_FACTOR_IGNORE_SMALLER_THAN_SIZE && cur >= target {
                        break;
                    }
                    let deadtime = deadtime(timer);
                    select! {
                        a = self.log_size.changed() => {
                            a?;
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        },
                        () = tokio::time::sleep(deadtime) => { break; },
                    };
                }
            }
        }

        *self.last_size.lock() = *self.log_size.borrow();
        *self.last_compact.lock() = Some(Instant::now());

        Ok(())
    }
}