#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::backend::reqwest::*;

use crate::api::*;

pub fn reqwest(system: System, request: Request) -> AsyncResult<Result<Response, i32>> {
    system.spawn_shared(move || async move {
        let url = request.url;
        let method = request.method;
        let headers = request.headers;
        let data = request.body;

        debug!("executing HTTP {}", method);

        let resp = system.reqwest(&url, &method, headers, data).await?;
        let status = resp.status;

        let headers = Vec::new();
        // we can't implement this as the method resp.headers().keys() is missing!
        // how else are we going to parse the headers

        debug!("received {} bytes", resp.data.len());
        let resp = Response {
            ok: resp.ok,
            redirected: resp.redirected,
            status: resp.status,
            status_text: resp.status_text,
            headers,
            data: Some(resp.data),
        };
        debug!("response status {}", status);
        Ok(resp)
    })
}
