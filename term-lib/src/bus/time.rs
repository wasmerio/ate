use wasm_bus::backend::time::Sleep;

use crate::api::*;

pub fn sleep(system: System, request: Sleep) -> AsyncResult<()> {
    system.spawn_shared(move || async move {
        let _ = system.sleep(request.duration_ms as i32).await;
    })
}
