#![allow(dead_code)]
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus_reqwest::api;
use wasm_bus_reqwest::prelude::*;
use wasmer_vbus::InstantInvocation;
use wasmer_vbus::VirtualBusInvoked;

use crate::api::*;

pub fn reqwest(
    system: System,
    request: api::ReqwestMakeRequest,
) -> Box<dyn VirtualBusInvoked> {
    let url = request.url;
    let method = request.method;
    let headers = request.headers;
    let data = request.body;

    let options = request.options;
    let options = ReqwestOptions {
        gzip: options.gzip,
        cors_proxy: options.cors_proxy.clone(),
    };

    debug!("executing HTTP {}", method);

    let ret = system.reqwest(&url, &method, options, headers, data);
    let result = system.spawn_shared(move || async move {
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
    });

    Box::new(InstantInvocation::call(Box::new(result)))
}
