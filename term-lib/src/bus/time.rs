#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::backend::time::Sleep;

use crate::api::*;

pub fn sleep(system: System, request: Sleep) -> AsyncResult<()> {
    system.sleep(request.duration_ms as i32)
}
