#[allow(unused_imports, dead_code)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use error_chain::bail;
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::Weak;
use std::convert::Infallible;
use fxhash::FxHashMap;
use tokio::sync::Mutex;
use url::Url;
use bytes::Bytes;
use std::time::Instant;
use std::ops::Deref;
use std::collections::hash_map::Entry as StdEntry;
use ttl_cache::TtlCache;
use std::time::Duration;
use std::net::SocketAddr;

use hyper;
use hyper::service::{make_service_fn, service_fn};
use hyper::Request;
use hyper::Response;
use hyper::Body;
use hyper::Method;
use hyper::StatusCode;
use hyper::server::conn::AddrStream;
use hyper::upgrade::Upgraded;
use hyper::header::HeaderValue;
use hyper_tungstenite::WebSocketStream;

use ate::prelude::*;
use ate_files::prelude::*;

use crate::model::WebConf;

use super::error::WebServerError;
use super::error::WebServerErrorKind;
use super::conf::*;
use super::builder::*;

pub struct ServerWebConf
{   
    web_conf: Option<WebConf>,
    web_conf_when: Option<Instant>,
}

#[async_trait]
pub trait ServerCallback: Send + Sync
{
    async fn web_socket(&self, _ws: WebSocketStream<Upgraded>, _sock_addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>>
    {
        Ok(())
    }
}

pub struct Server
{
    remote: Url,
    chains: Mutex<TtlCache<String, Arc<FileAccessor>>>,
    registry: Registry,
    web_conf: Mutex<FxHashMap<String, ServerWebConf>>,
    server_conf: ServerConf,
    callback: Option<Arc<dyn ServerCallback>>,
}

async fn process(server: Arc<Server>, listen: Arc<ServerListen>, req: Request<Body>, sock_addr: SocketAddr) -> Result<Response<Body>, hyper::Error> {
    let path = req.uri().path().to_string();
    match server.process(req, sock_addr, listen.deref()).await {
        Ok(resp) => {
            trace!("res: status={}", resp.status().as_u16());
            Ok(resp)
        },
        Err(WebServerError(WebServerErrorKind::FileSystemError(FileSystemErrorKind::NoAccess), _)) => {
            let err = format!("Access Denied - {}\n", path);
            let mut resp = Response::new(Body::from(err));
            *resp.status_mut() = StatusCode::FORBIDDEN;
            trace!("res: status={}", resp.status().as_u16());
            return Ok(resp);
        }
        Err(err) => {
            let mut resp = Response::new(Body::from(err.response_body()));
            *resp.status_mut() = err.status_code();
            trace!("res: status={}", resp.status().as_u16());
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
                remote: builder.remote,
                chains: Mutex::new(TtlCache::new(usize::MAX)),
                web_conf: Mutex::new(FxHashMap::default()),
                server_conf: builder.conf,
                callback: builder.callback,
            }
        )
    }

    pub async fn run(self: &Arc<Self>) -> Result<(), Box<dyn std::error::Error>>
    {
        let mut joins = Vec::new();
        for listen in self.server_conf.listen.iter() {
            let make_service = {
                let server = Arc::clone(self);
                let listen = Arc::new(listen.clone());
                make_service_fn(move |conn: &AddrStream| {
                    let addr = conn.remote_addr();
                    let server = server.clone();
                    let listen = listen.clone();
                    async move { Ok::<_, Infallible>(service_fn(move |req| process(server.clone(), listen.clone(), req, addr))) }
                })
            };

            let server = hyper::Server::bind(&listen.addr)
                .http1_preserve_header_case(true)
                .http1_title_case_headers(true)
                .serve(make_service);    
            println!("Listening on {}", listen.addr);
            joins.push(server);
        }

        {
            let server = Arc::clone(self);
            TaskEngine::spawn(async move {
                let server = {
                    let s = Arc::downgrade(&server);
                    drop(server);
                    s
                };
                let mut n = 0u32;
                loop {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    let server = match Weak::upgrade(&server) {
                        Some(a) => a,
                        None => break
                    };
                    n += 1;
                    if n > 30 {
                        server.house_keeping().await;
                        n = 0;
                    }
                }
            })
        }

        for res in futures::future::join_all(joins).await
        {
            if let Err(e) = res {
                eprintln!("server error: {}", e);
            }
        }
        Ok(())
    }

    async fn house_keeping(&self) {
        let mut lock = self.chains.lock().await;
        lock.iter();    // this will run the remove_expired function
    }

    pub(crate) fn get_host(&self, req: &Request<Body>) -> Result<String, WebServerError>
    {
        match req.headers().get("Host") {
            Some(a) => Ok(a.to_str()?.to_string()),
            None => { bail!(WebServerErrorKind::UnknownHost); }
        }
    }

    pub(crate) async fn get_chain(&self, host: &str) -> Result<Arc<FileAccessor>, WebServerError>
    {
        // Now get the chain for this host
        let host = host.to_string();
        let chain = {
            let mut chains = self.chains.lock().await;
            if let Some(ret) = chains.remove(&host) {
                chains.insert(host, Arc::clone(&ret), self.server_conf.ttl);
                ret
            } else {
                let key = ChainKey::from(format!("{}/www", host));
                let chain = self.registry.open(&self.remote, &key).await?;
                let accessor = Arc::new(
                    FileAccessor::new(
                        chain.as_arc(),
                        Some(host.clone()),
                        AteSessionType::User(AteSessionUser::default()),
                        TransactionScope::Local,
                        TransactionScope::None,
                        false,
                        false
                    ).await
                );
                chains.insert(host, Arc::clone(&accessor), self.server_conf.ttl);
                accessor
            }
        };

        Ok(chain)
    }

    pub(crate) async fn get_file(&self, host: &str, path: &str) -> Result<Option<Bytes>, WebServerError> {
        let path = path.to_string();
        let context = RequestContext {
            uid: 0u32,
            gid: 0u32,
        };
        
        let chain = self.get_chain(host).await?;
        Ok(
            match chain.search(&context, path.as_str()).await? {
                Some(a) => {
                    let flags = libc::O_RDONLY as u32;
                    let oh = chain.open(&context, a.ino, flags).await?;
                    let data = chain.read(&context, a.ino, oh.fh, 0, u32::MAX).await?;
                    Some(data)
                },
                None => {
                    None
                }
            }
        )
    }

    pub(crate) async fn get_conf(&self, host: &str) -> Result<WebConf, WebServerError> {
        let host = host.to_string();

        let mut lock = self.web_conf.lock().await;
        let lock = match lock.entry(host.clone()) {
            StdEntry::Occupied(a) => a.into_mut(),
            StdEntry::Vacant(a) => {
                a.insert(ServerWebConf {
                    web_conf: None,
                    web_conf_when: None
                })
            }
        };

        let trigger = match &lock.web_conf_when {
            Some(a) if a.elapsed().as_millis() > 4000u128 => true,
            None => true,
            _ => false,
        };
        if trigger {
            lock.web_conf_when = Some(Instant::now());
            lock.web_conf =
                match self.get_file(host.as_str(), "web.yaml")
                    .await.ok().flatten()
                {
                    Some(data) => {
                        let data = String::from_utf8_lossy(&data[..]);
                        Some(
                            serde_yaml::from_str::<WebConf>(&data)
                                .map_err(|err| WebServerError::from_kind(WebServerErrorKind::BadConfiguration(err.to_string())))?
                        )
                    },
                    None => None,   
                };
        }

        if let Some(conf) = &lock.web_conf {
            match serde_yaml::to_string(conf) {
                Ok(conf) => trace!("web-conf: {}", conf),
                Err(err) => trace!("web-conf-err: {}", err)
            };
            Ok(conf.clone())
        } else {
            trace!("web-conf-default");
            Ok(WebConf::default())
        }
    }

    pub(crate) async fn process_redirect(&self, req: Request<Body>, listen: &ServerListen, redirect: &str) -> Result<Response<Body>, WebServerError>
    {
        let mut uri = http::Uri::builder()
            .authority(redirect);
        if let Some(scheme) = req.uri().scheme() {
            uri = uri.scheme(scheme.clone());
        } else if listen.tls {
            uri = uri.scheme("https");
        } else {
            uri = uri.scheme("http");
        }
        if let Some(path_and_query) = req.uri().path_and_query() {
            uri = uri.path_and_query(path_and_query.clone());
        }
        let uri = match uri.build() {
            Ok(uri) => uri,
            Err(err) => {
                bail!(WebServerErrorKind::BadRequest(err.to_string()))
            }
        }.to_string();
        
        let mut resp = Response::new(Body::from(crate::helper::redirect_body(&uri)));
        resp.headers_mut().append("Location", HeaderValue::from_str(uri.as_str())?);
        *resp.status_mut() = StatusCode::PERMANENT_REDIRECT;
        return Ok(resp);
    }

    pub(crate) async fn process_get(&self, host: &str, path: &str, is_head: bool) -> Result<Option<Response<Body>>, WebServerError> {
        if let Some(data) = self.get_file(host, path).await? {
            let len_str = data.len().to_string();

            let mut resp = if is_head {
                Response::new(Body::empty())
            } else {
                Response::new(Body::from(data))
            };
            resp.headers_mut().append("Content-Length", HeaderValue::from_str(len_str.as_str())?);
            *resp.status_mut() = StatusCode::OK;
            Ok(Some(resp))
        } else {
            Ok(None)
        }
    }

    pub(crate) async fn process_get_with_default(&self, host: &str, path: &str, is_head: bool, default_page: Option<&String>) -> Result<Response<Body>, WebServerError> {
        match self.process_get(host, path, is_head).await? {
            Some(a) => {
                return Ok(a);
            },
            None if path.len() == 0 || path == "/" => {
                if let Some(default_page) = default_page {
                    if let Some(ret) = self.process_get(host, default_page.as_str(), is_head).await? {
                        return Ok(ret);
                    }
                }
            },
            None => {}
        }

        let data = format!("File Not Found - {}\n", path);
        let mut resp = Response::new(Body::from(data));
        *resp.status_mut() = StatusCode::NOT_FOUND;
        Ok(resp)
    }

    pub(crate) async fn process(&self, req: Request<Body>, sock_addr: SocketAddr, listen: &ServerListen) -> Result<Response<Body>, WebServerError> {
        trace!("req: {:?}", req);

        if hyper_tungstenite::is_upgrade_request(&req) {
            return self.process_upgrade(req, sock_addr).await;
        }

        let is_head = req.method() == Method::HEAD;
        let host = self.get_host(&req)?;
        let conf = self.get_conf(host.as_str()).await?;

        let ret = self.process_internal(req, listen, &conf).await;
        match ret {
            Ok(a) => Ok(a),
            Err(err) => {
                let page = conf.status_pages.get(&err.status_code().as_u16()).map(|a| a.clone());
                if let Some(page) = page {
                    if let Some(ret) = self.process_get(host.as_str(), page.as_str(), is_head).await? {
                        return Ok(ret);
                    }
                }
                return Err(err);
            }
        }
    }

    pub(crate) async fn process_upgrade(&self, req: Request<Body>, sock_addr: SocketAddr) -> Result<Response<Body>, WebServerError>
    {
        if let Some(callback) = &self.callback {
            let callback = Arc::clone(callback);
            let (response, websocket) = hyper_tungstenite::upgrade(req, None)?;
            TaskEngine::spawn(async move {
                match websocket.await {
                    Ok(websocket) => {
                        let ret = callback.web_socket(websocket, sock_addr).await;
                        if let Err(err) = ret {
                            error!("web socket failed - {}", err);
                        }
                    },
                    Err(err) => {
                        error!("web socket failed - {}", err);
                    }
                }                
            });
            Ok(response)
        } else {
            Err(WebServerErrorKind::BadRequest("websockets are not supported".to_string()).into())
        }
    }

    pub(crate) async fn process_internal(&self, req: Request<Body>, listen: &ServerListen, conf: &WebConf) -> Result<Response<Body>, WebServerError> {
        if let Some(redirect) = conf.redirect.as_ref() {
            return self.process_redirect(req, listen, &redirect).await;
        }

        let host = self.get_host(&req)?;
        let is_head = req.method() == Method::HEAD;

        match req.method() {
            &Method::HEAD | &Method::GET => {
                let path = req.uri().path();
                self.process_get_with_default(host.as_str(), path, is_head, conf.default_page.as_ref()).await
            },
            _ => {
                let mut resp = Response::new(Body::from(StatusCode::METHOD_NOT_ALLOWED.as_str()));
                *resp.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
                Ok(resp)
            }
        }
    }
}