#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::api::*;

pub fn sleep(system: System, duration_ms: u128) -> AsyncResult<()> {
    system.sleep(duration_ms)
}
