#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::backend::reqwest::*;

use crate::api::*;

pub fn reqwest(system: System, request: Request) -> AsyncResult<Result<Response, i32>> {
    let url = request.url;
    let method = request.method;
    let headers = request.headers;
    let data = request.body;

    debug!("executing HTTP {}", method);

    system.reqwest(&url, &method, headers, data)
}
