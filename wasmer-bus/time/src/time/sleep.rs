#[cfg(target_os = "wasi")]
pub async fn sleep(duration: std::time::Duration) {
    let duration_ms = duration.as_millis();
    let _ = crate::api::TimeClient::new(super::WAPM_NAME)
        .sleep(duration_ms)
        .await;
}

#[cfg(not(target_os = "wasi"))]
pub use tokio::time::sleep;
