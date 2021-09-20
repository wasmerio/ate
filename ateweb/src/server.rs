#[allow(unused_imports, dead_code)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use error_chain::bail;
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::Weak;
use std::convert::Infallible;
use fxhash::FxHashMap;
use tokio::sync::Mutex;
use std::time::Instant;
use std::ops::Deref;
use std::collections::hash_map::Entry as StdEntry;
use std::time::Duration;
use std::net::SocketAddr;
use tokio::net::TcpListener;

use hyper;
use hyper::service::{make_service_fn, service_fn};
use hyper::Request;
use hyper::Response;
use hyper::Body;
use hyper::Method;
use hyper::StatusCode;
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
use super::acceptor::*;
use super::stream::*;
use super::acme::AcmeResolver;
use super::repo::*;
use super::model::*;

pub struct ServerWebConf
{   
    web_conf: WebConf,
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
    repo: Arc<Repository>,
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
    pub(crate) async fn new(builder: ServerBuilder) -> Result<Arc<Server>, AteError>
    {
        let registry = Arc::new(Registry::new(&builder.conf.cfg_ate).await);
        let repo = Repository::new(
            &registry,
            builder.remote.clone(),
            builder.auth_url.clone(),
            builder.web_key.clone(),
            builder.conf.ttl
        ).await?;
        Ok(
            Arc::new(
                Server {
                    repo,
                    web_conf: Mutex::new(FxHashMap::default()),
                    server_conf: builder.conf,
                    callback: builder.callback,
                }
            )
        )
    }

    pub async fn run(self: &Arc<Self>) -> Result<(), Box<dyn std::error::Error>>
    {
        trace!("running web server");

        let acme = AcmeResolver::new(&self.repo).await?;

        let mut joins = Vec::new();
        for listen in self.server_conf.listen.iter() {
            let make_service = {
                let server = Arc::clone(self);
                let listen = Arc::new(listen.clone());
                make_service_fn(move |conn: &HyperStream| {
                    let addr = conn.remote_addr().clone();
                    let server = server.clone();
                    let listen = listen.clone();
                    async move { Ok::<_, Infallible>(service_fn(move |req| process(server.clone(), listen.clone(), req, addr))) }
                })
            };

            let acme = acme.clone();
            let tcp_listener = TcpListener::bind(&listen.addr).await?;
            let acceptor = HyperAcceptor::new(tcp_listener, acme, listen.tls);
            let server = hyper::Server::builder(acceptor)
                .http1_preserve_header_case(true)
                .http1_title_case_headers(true)
                .serve(make_service);    
            println!("Listening on {}", listen.addr);
            joins.push(server);
        }

        // This next background thread will terminate any chains that have gone
        // out-of-scope due to expired TTL (caching cleanup)
        {
            let ttl = self.server_conf.ttl.as_secs();
            let ttl_check = u64::min(ttl, 30u64);
            let server = Arc::clone(self);
            TaskEngine::spawn(async move {
                let server = {
                    let s = Arc::downgrade(&server);
                    drop(server);
                    s
                };
                let mut n = 0u64;
                loop {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    let server = match Weak::upgrade(&server) {
                        Some(a) => a,
                        None => break
                    };
                    n += 1;
                    if n >= ttl_check {
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
        self.repo.house_keeping().await;
    }

    pub(crate) fn get_host(&self, req: &Request<Body>) -> Result<String, WebServerError>
    {
        if let Some(host) = req.uri().host() {
            return Ok(host.to_string());
        }
        match req.headers().get("Host") {
            Some(a) => Ok(a.to_str()?.to_string()),
            None => { bail!(WebServerErrorKind::UnknownHost); }
        }
    }

    pub(crate) async fn get_conf(&self, host: &str) -> Result<WebConf, WebServerError> {
        let host = host.to_string();

        let mut lock = self.web_conf.lock().await;
        let conf = match lock.entry(host.clone()) {
            StdEntry::Occupied(a) => a.into_mut(),
            StdEntry::Vacant(a) => {
                a.insert(ServerWebConf {
                    web_conf: WebConf::default(),
                    web_conf_when: None
                })
            }
        };

        let trigger = match &conf.web_conf_when {
            Some(a) if a.elapsed().as_millis() > 4000u128 => true,
            None => true,
            _ => { return Ok(conf.web_conf.clone()); },
        };
        if trigger {
            conf.web_conf_when = Some(Instant::now());
            conf.web_conf =
                match self.repo.get_file(host.as_str(), WEB_CONF_FILES_CONF)
                    .await.ok().flatten()
                {
                    Some(data) => {
                        let data = String::from_utf8_lossy(&data[..]);
                        serde_yaml::from_str::<WebConf>(&data)
                            .map_err(|err| WebServerError::from_kind(WebServerErrorKind::BadConfiguration(err.to_string())))?
                    },
                    None => {
                        let mut ret = WebConf::default();
                        ret.default_page = Some("index.html".to_string());
                        ret.force_https = true;

                        if let Some(ret_str) = serde_yaml::to_string(&ret).ok() {
                            let err = self.repo.set_file(host.as_str(), WEB_CONF_FILES_CONF, ret_str.as_bytes()).await;
                            if let Err(err) = err {
                                warn!("failed to save default web.yaml - {}", err);
                            }
                        }
                        ret
                    },
                };
        }

        match serde_yaml::to_string(&conf.web_conf) {
            Ok(conf) => trace!("web-conf: {}", conf),
            Err(err) => trace!("web-conf-err: {}", err)
        };
        Ok(conf.web_conf.clone())
    }

    pub(crate) async fn force_https(&self, req: Request<Body>) -> Result<Response<Body>, WebServerError> {
        let host = match req.uri().authority() {
            Some(a) => a.to_string(),
            None => {
                match req.headers().get("Host") {
                    Some(a) => a.to_str()?.to_string(),
                    None => {
                        bail!(WebServerErrorKind::BadRequest("unknown host address needed for redirect to https".to_string()))
                    }
                }
            }
        };
        let mut uri = http::Uri::builder()
            .authority(host.as_str())
            .scheme("https");
        if let Some(path_and_query) = req.uri().path_and_query() {
            uri = uri.path_and_query(path_and_query.clone());
        }
        let uri = match uri.build() {
            Ok(uri) => uri,
            Err(err) => {
                bail!(WebServerErrorKind::BadRequest(err.to_string()))
            }
        }.to_string();
        
        self.process_redirect(uri.as_str()).await
    }

    pub(crate) async fn process_redirect_host(&self, req: Request<Body>, listen: &ServerListen, redirect: &str) -> Result<Response<Body>, WebServerError>
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
        
        self.process_redirect(uri.as_str()).await
    }

    pub(crate) async fn process_redirect(&self, uri: &str) -> Result<Response<Body>, WebServerError>
    {
        let mut resp = Response::new(Body::from(crate::helper::redirect_body(uri)));
        resp.headers_mut().append("Location", HeaderValue::from_str(uri)?);
        *resp.status_mut() = StatusCode::PERMANENT_REDIRECT;
        return Ok(resp);
    }

    pub(crate) fn sanitize(&self, mut path: &str) -> Result<(), WebServerError> {
        while path.starts_with("/") {
            path = &path[1..];
        }
        if path.contains("..") {
            bail!(WebServerErrorKind::BadRequest("Accessing parent directories is forbidden".to_string()));
        }
        if path.starts_with(WEB_CONF_FILES) {
            bail!(WebServerErrorKind::BadRequest("Accessing configuration files is forbidden".to_string()));
        }
        Ok(())
    }

    pub(crate) async fn process_get(&self, host: &str, path: &str, is_head: bool) -> Result<Option<Response<Body>>, WebServerError> {
        self.sanitize(path)?;
        if let Some(data) = self.repo.get_file(host, path).await? {
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
        self.sanitize(path)?;
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
            return self.process_redirect_host(req, listen, &redirect).await;
        }

        if conf.force_https && listen.tls == false {
            return self.force_https(req).await;
        }

        let host = self.get_host(&req)?;
        let is_head = req.method() == Method::HEAD;

        match req.method() {
            &Method::HEAD | &Method::GET => {
                let path = req.uri().path();
                self.sanitize(path)?;
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