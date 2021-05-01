#![cfg(test)]
use std::time::Duration;
use std::time::Instant;
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
        let (tx, mut rx) = CompactState::new(CompactMode::Never, 0);
        let wait = rx.wait_for_compact();
        tx.log_size.send(100u64)?;
        timeout(Duration::from_millis(20), wait).await
            .expect_err("The never event should never be triggered");

        // Test the timer trigger (negative)
        let (_tx, mut rx) = CompactState::new(CompactMode::Timer(Duration::from_millis(100)), 0);
        let wait = rx.wait_for_compact();
        timeout(Duration::from_millis(50), wait).await
            .expect_err("The timer event should not be triggered");

        // Test the timer trigger (positive)
        let (_tx, mut rx) = CompactState::new(CompactMode::Timer(Duration::from_millis(100)), 0);
        let wait = rx.wait_for_compact();
        let start = Instant::now();
        timeout(Duration::from_millis(150), wait).await
            .expect("This should not timeout")?;
        let elapsed = start.elapsed();
        assert!(elapsed > Duration::from_millis(100), "The timer should have waited 100 milliseconds");

        // Test the modify trigger (negative)
        let (_tx, mut rx) = CompactState::new(CompactMode::Modified, 0);
        let wait = rx.wait_for_compact();
        timeout(Duration::from_millis(20), wait).await
            .expect_err("The modify event should not be triggered");

        // Test the modify trigger (positive)
        let (tx, mut rx) = CompactState::new(CompactMode::Modified, 0);
        let wait = rx.wait_for_compact();
        let _tx = tokio::spawn(async move { tokio::time::sleep(Duration::from_millis(10)).await; let _ = tx.log_size.send(100u64); tx });
        timeout(Duration::from_millis(100), wait).await
            .expect("This should not timeout")?;

        // Test the growth size trigger (negative I)
        let (_tx, mut rx) = CompactState::new(CompactMode::GrowthSize(500), 1000);
        let wait = rx.wait_for_compact();
        timeout(Duration::from_millis(20), wait).await
            .expect_err("The growth size event should not be triggered");

        // Test the growth size trigger (negative II)
        let (tx, mut rx) = CompactState::new(CompactMode::GrowthSize(500), 1000);
        let wait = rx.wait_for_compact();
        tx.log_size.send(100u64)?;
        let _tx = tokio::spawn(async move { tokio::time::sleep(Duration::from_millis(10)).await; let _ = tx.log_size.send(1100u64); tx });
        timeout(Duration::from_millis(50), wait).await
            .expect_err("The modify event should not be triggered");

        // Test the growth size trigger (positive)
        let (tx, mut rx) = CompactState::new(CompactMode::GrowthSize(500), 1000);
        let wait = rx.wait_for_compact();
        let _tx = tokio::spawn(async move { tokio::time::sleep(Duration::from_millis(10)).await; let _ = tx.log_size.send(1400u64); let _ = tx.log_size.send(1600u64); tx });
        timeout(Duration::from_millis(100), wait).await
            .expect("This should not timeout")?;
        
        // Test the growth or timer trigger (negative I)
        let (_tx, mut rx) = CompactState::new(CompactMode::GrowthSizeOrTimer { growth: 500, timer: Duration::from_millis(100) }, 1000);
        let wait = rx.wait_for_compact();
        timeout(Duration::from_millis(20), wait).await
            .expect_err("The growth or timer event should not be triggered");

        // Test the growth or timer trigger (positive I via timer)
        let (_tx, mut rx) = CompactState::new(CompactMode::GrowthSizeOrTimer { growth: 500, timer: Duration::from_millis(100) }, 1000);
        let wait = rx.wait_for_compact();
        let start = Instant::now();
        timeout(Duration::from_millis(150), wait).await
            .expect("This growth or timer event should not have timeed out")?;
        let elapsed = start.elapsed();
        assert!(elapsed > Duration::from_millis(100), "The growth or timer event should have waited 100 milliseconds");
        
        // Test the growth of timer trigger (positive II via growth)
        let (tx, mut rx) = CompactState::new(CompactMode::GrowthSizeOrTimer { growth: 500, timer: Duration::from_millis(100) }, 1000);
        let wait = rx.wait_for_compact();
        let start = Instant::now();
        let _tx = tokio::spawn(async move { tokio::time::sleep(Duration::from_millis(10)).await; let _ = tx.log_size.send(2000u64); tx });
        timeout(Duration::from_millis(50), wait).await
            .expect("This growth or timer event should have triggered")?;
        let elapsed = start.elapsed();
        assert!(elapsed < Duration::from_millis(100), "The growth or timer event should not have waited 100 milliseconds");
        
        Ok(())
    })
}