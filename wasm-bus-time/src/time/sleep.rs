use std::time::Duration;

use super::*;

pub async fn sleep(duration: Duration) {
    let duration_ms = duration.as_millis();
    let _ = crate::api::Time::sleep(WAPM_NAME, duration_ms).join().await;
}
