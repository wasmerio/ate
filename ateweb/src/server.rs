#[allow(unused_imports, dead_code)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use error_chain::bail;
use std::sync::Arc;
use std::convert::Infallible;
use fxhash::FxHashMap;
use tokio::sync::Mutex;
use std::collections::hash_map::Entry;
use url::Url;

use hyper;
use hyper::service::{make_service_fn, service_fn};
use hyper::Request;
use hyper::Response;
use hyper::Body;
use hyper::Method;
use hyper::StatusCode;
use hyper::header::HeaderValue;

use ate::prelude::*;
use ate_files::prelude::*;

use super::error::WebServerError;
use super::error::WebServerErrorKind;
use super::conf::*;
use super::builder::*;

pub struct Server
{
    conf: ServerConf,
    registry: Registry,
    remote: Url,
    chains: Mutex<FxHashMap<String, Arc<FileAccessor>>>
}

async fn process(server: Arc<Server>, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let path = req.uri().path().to_string();
    match server.process(req).await {
        Ok(a) => Ok(a),
        Err(WebServerError(WebServerErrorKind::FileSystemError(FileSystemErrorKind::NoAccess), _)) => {
            let err = format!("Access Denied - {}\n", path);
            let mut resp = Response::new(Body::from(err));
            *resp.status_mut() = StatusCode::FORBIDDEN;
            return Ok(resp);
        }
        Err(err) => {
            let mut resp = Response::new(Body::from(err.response_body()));
            *resp.status_mut() = err.status_code();
            return Ok(resp);
        }
    }
}

impl Server
{
    pub(crate) async fn new(builder: ServerBuilder) -> Arc<Server>
    {
        Arc::new(
            Server {
                registry: Registry::new(&builder.conf.cfg_ate).await,
                conf: builder.conf,
                remote: builder.remote,
                chains: Mutex::new(FxHashMap::default()),
            }
        )
    }

    pub async fn run(self: &Arc<Self>) -> Result<(), Box<dyn std::error::Error>>
    {
        let mut joins = Vec::new();
        for listen in self.conf.listen.iter() {
            let server = Arc::clone(self);
            let make_service = make_service_fn(move |_| {
                let server = server.clone();
                async move { Ok::<_, Infallible>(service_fn(move |req| process(server.clone(), req))) }
            });

            let server = hyper::Server::bind(&listen.addr)
                .http1_preserve_header_case(true)
                .http1_title_case_headers(true)
                .serve(make_service);    
            println!("Listening on {}", listen.addr);
            joins.push(server);
        }

        for res in futures::future::join_all(joins).await
        {
            if let Err(e) = res {
                eprintln!("server error: {}", e);
            }
        }
        Ok(())
    }

    pub(crate) async fn process(&self, req: Request<Body>) -> Result<Response<Body>, WebServerError> {
        trace!("req: {:?}", req);

        // Determine the host that this method relates to
        let host = match req.headers().get("Host") {
            Some(a) => a.to_str()?,
            None => { bail!(WebServerErrorKind::UnknownHost); }
        };

        // Now get the chain for this host
        let chain = {
            let mut lock = self.chains.lock().await;
            match lock.entry(host.to_string()) {
                Entry::Occupied(a) => {
                    Arc::clone(a.get())
                },
                Entry::Vacant(a) => {
                    let key = ChainKey::from(format!("{}/www", host));
                    let chain = self.registry.open(&self.remote, &key).await?;
                    let accessor = Arc::new(
                        FileAccessor::new(
                            chain.as_arc(),
                            Some(host.to_string()),
                            AteSessionType::User(AteSessionUser::default()),
                            TransactionScope::Local,
                            TransactionScope::None,
                            false,
                            false
                        ).await
                    );
                    a.insert(Arc::clone(&accessor));
                    accessor
                }
            }
        };

        match req.method() {
            &Method::HEAD | &Method::GET =>
            {
                let path = req.uri().path().to_string();
                let context = RequestContext {
                    uid: 0u32,
                    gid: 0u32,
                };
                
                match chain.search(&context, path.as_str()).await? {
                    Some(a) => {
                        let flags = libc::O_RDONLY as u32;
                        let oh = chain.open(&context, a.ino, flags).await?;
                        let data = chain.read(&context, a.ino, oh.fh, 0, u32::MAX).await?;
                        let len_str = data.len().to_string();

                        let mut resp = if req.method() == &Method::HEAD {
                            Response::new(Body::empty())
                        } else {
                            Response::new(Body::from(data))
                        };
                        resp.headers_mut().append("Content-Length", HeaderValue::from_str(len_str.as_str())?);
                        *resp.status_mut() = StatusCode::OK;
                        Ok(resp)
                    },
                    None => {
                        let data = format!("File Not Found - {}\n", path);
                        let mut resp = Response::new(Body::from(data));
                        *resp.status_mut() = StatusCode::NOT_FOUND;
                        Ok(resp)
                    }
                }
            },
            _ => {
                let mut resp = Response::new(Body::from(StatusCode::METHOD_NOT_ALLOWED.as_str()));
                *resp.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
                Ok(resp)
            }
        }
    }
}