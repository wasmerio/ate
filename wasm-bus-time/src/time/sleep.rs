use std::time::Duration;

use super::*;

pub async fn sleep(duration: Duration) {
    let duration_ms = duration.as_millis();
    let _ = crate::api::TimeClient::new(WAPM_NAME)
        .sleep(duration_ms)
        .await;
}
