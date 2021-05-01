use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::watch;
use parking_lot::Mutex;
use tokio::select;    

/// # Compaction State Machine
/// 
/// State machine that will trigger a compaction only when a particular set
/// of states has been reached.

// Specifies when a compaction event on a chain will occur.
#[derive(Debug, Clone, Copy)]
pub enum CompactMode
{
    // Compaction will never occur which effectivily means this chain is immutable
    Never,
    // Comapction will be triggered when the chain is modified in any way
    Modified,
    // Compaction will occur whenever a timer duration has been reached
    Timer(Duration),
    // Compaction will occur whenever growth exceeds a particular percentage
    GrowthFactor(f32),
    // Compaction will occur whenever growth exceeds a particular percentage or the timer is triggered
    GrowthFactorOrTimer {
        growth: f32,
        timer: Duration
    },
    // Compaction will occur whever the chain size increases by a certain absolute amount in bytes
    GrowthSize(u64),
    // Compaction will occur whever the chain size increases by a certain absolute amount in bytes or the timer is triggered
    GrowthSizeOrTimer {
        growth: u64,
        timer: Duration
    },
}

pub(crate) struct CompactState
{
    pub log_size: watch::Receiver<u64>,
    pub last_size: Arc<Mutex<u64>>,
    pub last_compact: Arc<Mutex<Option<Instant>>>
}

pub(crate) struct CompactNotifications
{
    pub log_size: watch::Sender<u64>,
}

impl CompactState
{
    pub fn new(size: u64) -> (CompactNotifications, CompactState) {
        let (modified_tx, modified_rx) = watch::channel::<u64>(size);

        (
            CompactNotifications {
                log_size: modified_tx,
            },
            CompactState {
                log_size: modified_rx,
                last_size: Arc::new(Mutex::new(size)),
                last_compact: Arc::new(Mutex::new(None)),
            },
        )
    }

    pub async fn wait_for_compact(&mut self, mode: CompactMode) -> Result<(), watch::error::RecvError> {
        loop {
            let initial_size = {
                *self.last_size.lock()
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

            match mode {
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
                    if *self.log_size.borrow() >= target {
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
                    if *self.log_size.borrow() >= target {
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

#[cfg(test)]
mod tests 
{
    use tokio::time::timeout;
    use tokio::runtime::Runtime;
    use crate::error::AteError;
    use super::*;

    #[test]
    pub fn test_compact_state_machine() -> Result<(), AteError> {
        crate::utils::bootstrap_env();

        let rt = Runtime::new().unwrap();

        rt.block_on(async
        {
            // Test the never trigger (negative)
            let (tx, mut rx) = CompactState::new(0);
            let wait = rx.wait_for_compact(CompactMode::Never);
            tx.log_size.send(100u64)?;
            timeout(Duration::from_millis(20), wait).await
                .expect_err("The never event should never be triggered");

            // Test the timer trigger (negative)
            let (_tx, mut rx) = CompactState::new(0);
            let wait = rx.wait_for_compact(CompactMode::Timer(Duration::from_millis(100)));
            timeout(Duration::from_millis(50), wait).await
                .expect_err("The timer event should not be triggered");

            // Test the timer trigger (positive)
            let (_tx, mut rx) = CompactState::new(0);
            let wait = rx.wait_for_compact(CompactMode::Timer(Duration::from_millis(100)));
            let start = Instant::now();
            timeout(Duration::from_millis(150), wait).await
                .expect("This should not timeout")?;
            let elapsed = start.elapsed();
            assert!(elapsed > Duration::from_millis(100), "The timer should have waited 100 milliseconds");

            // Test the modify trigger (negative)
            let (_tx, mut rx) = CompactState::new(0);
            let wait = rx.wait_for_compact(CompactMode::Modified);
            timeout(Duration::from_millis(20), wait).await
                .expect_err("The modify event should not be triggered");

            // Test the modify trigger (positive)
            let (tx, mut rx) = CompactState::new(0);
            let wait = rx.wait_for_compact(CompactMode::Modified);
            let _tx = tokio::spawn(async move { tokio::time::sleep(Duration::from_millis(10)).await; let _ = tx.log_size.send(100u64); tx });
            timeout(Duration::from_millis(100), wait).await
                .expect("This should not timeout")?;

            // Test the growth size trigger (negative I)
            let (_tx, mut rx) = CompactState::new(1000);
            let wait = rx.wait_for_compact(CompactMode::GrowthSize(500));
            timeout(Duration::from_millis(20), wait).await
                .expect_err("The growth size event should not be triggered");

            // Test the growth size trigger (negative II)
            let (tx, mut rx) = CompactState::new(1000);
            let wait = rx.wait_for_compact(CompactMode::GrowthSize(500));
            tx.log_size.send(100u64)?;
            let _tx = tokio::spawn(async move { tokio::time::sleep(Duration::from_millis(10)).await; let _ = tx.log_size.send(1100u64); tx });
            timeout(Duration::from_millis(50), wait).await
                .expect_err("The modify event should not be triggered");

            // Test the growth size trigger (positive)
            let (tx, mut rx) = CompactState::new(1000);
            let wait = rx.wait_for_compact(CompactMode::GrowthSize(500));
            let _tx = tokio::spawn(async move { tokio::time::sleep(Duration::from_millis(10)).await; let _ = tx.log_size.send(1400u64); let _ = tx.log_size.send(1600u64); tx });
            timeout(Duration::from_millis(100), wait).await
                .expect("This should not timeout")?;
            
            // Test the growth or timer trigger (negative I)
            let (_tx, mut rx) = CompactState::new(1000);
            let wait = rx.wait_for_compact(CompactMode::GrowthSizeOrTimer { growth: 500, timer: Duration::from_millis(100) });
            timeout(Duration::from_millis(20), wait).await
                .expect_err("The growth or timer event should not be triggered");

            // Test the growth or timer trigger (positive I via timer)
            let (_tx, mut rx) = CompactState::new(1000);
            let wait = rx.wait_for_compact(CompactMode::GrowthSizeOrTimer { growth: 500, timer: Duration::from_millis(100) });
            let start = Instant::now();
            timeout(Duration::from_millis(150), wait).await
                .expect("This growth or timer event should not have timeed out")?;
            let elapsed = start.elapsed();
            assert!(elapsed > Duration::from_millis(100), "The growth or timer event should have waited 100 milliseconds");
            
            // Test the growth of timer trigger (positive II via growth)
            let (tx, mut rx) = CompactState::new(1000);
            let wait = rx.wait_for_compact(CompactMode::GrowthSizeOrTimer { growth: 500, timer: Duration::from_millis(100) });
            let start = Instant::now();
            let _tx = tokio::spawn(async move { tokio::time::sleep(Duration::from_millis(10)).await; let _ = tx.log_size.send(2000u64); tx });
            timeout(Duration::from_millis(50), wait).await
                .expect("This growth or timer event should have triggered")?;
            let elapsed = start.elapsed();
            assert!(elapsed < Duration::from_millis(100), "The growth or timer event should not have waited 100 milliseconds");
            
            Ok(())
        })
    }
}