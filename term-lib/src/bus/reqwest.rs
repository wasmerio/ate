#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus_reqwest::api;
use wasm_bus_reqwest::prelude::*;

use crate::api::*;

pub fn reqwest(
    system: System,
    request: api::ReqwestMakeRequest,
) -> AsyncResult<Result<api::Response, u32>> {
    let url = request.url;
    let options = request.options;
    let method = request.method;
    let headers = request.headers;
    let data = request.body;

    debug!("executing HTTP {}", method);

    let ret = system.reqwest(&url, &method, headers, data);
    system.spawn_shared(move || async move {
        if let Some(a) = ret.await {
            match a {
                Ok(a) => Ok(Response {
                    pos: a.pos,
                    data: a.data,
                    ok: a.ok,
                    redirected: a.redirected,
                    status: a.status,
                    status_text: a.status_text,
                    headers: a.headers,
                }),
                Err(err) => Err(err as u32),
            }
        } else {
            Err(crate::err::ERR_ECONNABORTED)
        }
    })
}
