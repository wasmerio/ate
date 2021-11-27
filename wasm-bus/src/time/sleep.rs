use std::time::Duration;

use crate::abi::call;
use crate::backend::time::Sleep;

use super::*;

pub async fn sleep(duration: Duration) {
    let duration_ms = duration.as_millis();
    call(WAPM_NAME.into(), Sleep { duration_ms })
        .invoke()
        .join::<()>()
        .await
        .unwrap();
}
