use async_trait::async_trait;
use bytes::Bytes;
use error_chain::bail;
use std::collections::HashMap;
use std::collections::hash_map::Entry as StdEntry;
use std::convert::Infallible;
use std::error::Error;
use std::net::SocketAddr;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;
#[allow(unused_imports)]
use std::sync::Weak;
#[allow(unused_imports)]
use std::time::Duration;
use std::time::Instant;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, instrument, span, trace, warn, event, Level};
#[cfg(feature = "wasmer-auth")]
use wasmer_auth::service::AuthService;
#[cfg(feature = "wasmer-auth")]
use wasmer_auth::cmd::gather_command;

use hyper;
use hyper::header::HeaderValue;
use hyper::service::{make_service_fn, service_fn};
use hyper::upgrade::Upgraded;
pub use hyper::Body;
use hyper::Method;
use hyper::Request;
use hyper::Response;
use hyper::StatusCode;
use hyper_tungstenite::WebSocketStream;

#[cfg(feature = "ate")]
use ate::prelude::*;
#[cfg(feature = "ate-files")]
use ate_files::prelude::*;
#[cfg(feature = "ate-files")]
use ate_files::repo::*;

use crate::model::WebConf;

use super::acceptor::*;
#[cfg(feature = "acme")]
use super::acme::AcmeResolver;
use super::builder::*;
use super::conf::*;
use super::error::WebServerError;
use super::error::WebServerErrorKind;
use super::model::*;
use super::stream::*;

pub struct ServerWebConf {
    web_conf: WebConf,
    #[allow(dead_code)]
    web_conf_when: Option<Instant>,
}

#[async_trait]
pub trait ServerCallback: Send + Sync {
    async fn web_socket(
        &self,
        _ws: WebSocketStream<Upgraded>,
        _sock_addr: SocketAddr,
        _uri: Option<http::Uri>,
        _headers: Option<http::HeaderMap>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    async fn post_request(
        &self,
        _body: Vec<u8>,
        _sock_addr: SocketAddr,
        _uri: http::Uri,
        _headers: http::HeaderMap,
    ) -> (Vec<u8>, StatusCode) {
        let msg = format!("Bad Request (Not Implemented)").as_bytes().to_vec();
        (msg, StatusCode::BAD_REQUEST)
    }

    async fn put_request(
        &self,
        _body: Vec<u8>,
        _sock_addr: SocketAddr,
        _uri: http::Uri,
        _headers: http::HeaderMap,
    ) -> (Vec<u8>, StatusCode) {
        let msg = format!("Bad Request (Not Implemented)").as_bytes().to_vec();
        (msg, StatusCode::BAD_REQUEST)
    }
}

pub struct Server {
    #[cfg(feature = "dfs")]
    repo: Arc<Repository>,
    web_conf: Mutex<HashMap<String, ServerWebConf>>,
    server_conf: ServerConf,
    callback: Option<Arc<dyn ServerCallback + 'static>>,
    mime: HashMap<String, String>,
    www_path: Option<String>,
}

async fn process(
    server: Arc<Server>,
    listen: Arc<ServerListen>,
    req: Request<Body>,
    sock_addr: SocketAddr,
) -> Result<Response<Body>, hyper::Error> {
    trace!("perf-checkpoint: hyper process (addr={})", sock_addr);

    #[allow(unused_variables)]
    let path = req.uri().path().to_string();
    match server.process(req, sock_addr, listen.deref()).await {
        Ok(resp) => {
            trace!("perf-checkpoint: hyper finished");
            trace!("res: status={}", resp.status().as_u16());
            Ok(resp)
        }
        #[cfg(feature = "dfs")]
        Err(WebServerError(
            WebServerErrorKind::FileSystemError(FileSystemErrorKind::NoAccess),
            _,
        )) => {
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

impl Server {
    pub(crate) async fn new(#[allow(unused_mut)] mut builder: ServerBuilder) -> Result<Arc<Server>, Box<dyn Error>>
    {
        // There are few more tweaks we need to make to the configuration
        #[cfg(feature = "dfs")]
        {
            builder.conf.cfg_ate.recovery_mode = RecoveryMode::ReadOnlySync;
            builder.conf.cfg_ate.backup_mode = BackupMode::None;
        }
        
        // Now we are ready
        #[cfg(feature = "dfs")]
        let registry = Arc::new(Registry::new(&builder.conf.cfg_ate).await);

        #[cfg(feature = "dfs")]
        let repo = Repository::new(
            &registry,
            builder.remote.clone(),
            builder.auth_url.clone(),
            Box::new(SessionFactory {
                #[cfg(feature = "wasmer-auth")]
                auth_url: builder.auth_url.clone(),
                #[cfg(feature = "dfs")]
                registry: registry.clone(),
                #[cfg(feature = "ate")]
                master_key: builder.web_master_key.clone(),
            }),
            builder.conf.ttl,
        )
        .await?;

        Ok(Arc::new(Server {
            #[cfg(feature = "dfs")]
            repo,
            web_conf: Mutex::new(HashMap::default()),
            server_conf: builder.conf,
            callback: builder.callback,
            mime: Server::init_mime(),
            www_path: builder.www_path,
        }))
    }

    pub async fn run(self: &Arc<Self>) -> Result<(), Box<dyn std::error::Error>> {
        trace!("running web server");

        #[cfg(feature = "acme")]
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
                    async move {
                        Ok::<_, Infallible>(service_fn(move |req| {
                            process(server.clone(), listen.clone(), req, addr)
                        }))
                    }
                })
            };

            #[cfg(feature = "acme")]
            let acme = acme.clone();
            let tcp_listener = TcpListener::bind(&listen.addr).await?;
            let acceptor = HyperAcceptor::new(
                tcp_listener,
                #[cfg(feature = "acme")]
                acme,
                #[cfg(feature = "tls")]
                listen.tls
            );
            let server = hyper::Server::builder(acceptor)
                .http1_preserve_header_case(true)
                .http1_title_case_headers(true)
                .serve(make_service);
            println!("Listening on {}", listen.addr);
            joins.push(server);
        }

        // This next background thread will terminate any chains that have gone
        // out-of-scope due to expired TTL (caching cleanup)
        #[cfg(feature = "dfs")]
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
                    ate::engine::sleep(Duration::from_secs(1)).await;
                    let server = match Weak::upgrade(&server) {
                        Some(a) => a,
                        None => break,
                    };
                    n += 1;
                    if n >= ttl_check {
                        server.house_keeping().await;
                        n = 0;
                    }
                }
            });
        }

        for res in futures::future::join_all(joins).await {
            if let Err(e) = res {
                eprintln!("server error: {}", e);
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    async fn house_keeping(&self) {
        #[cfg(feature = "dfs")]
        self.repo.house_keeping().await;
    }

    pub(crate) fn get_host(&self, req: &Request<Body>) -> Result<String, WebServerError> {
        if let Some(host) = req.uri().host() {
            return Ok(host.to_string());
        }
        match req.headers().get("Host") {
            Some(a) => {
                let mut host = a.to_str()?.to_string();
                if let Some((a, _)) = host.split_once(":") {
                    host = a.to_string();
                }
                Ok(host)
            },
            None => {
                bail!(WebServerErrorKind::UnknownHost);
            }
        }
    }

    pub(crate) async fn get_conf(&self, host: &str) -> Result<WebConf, WebServerError> {
        let host = host.to_string();

        let mut lock = self.web_conf.lock().await;
        let conf = match lock.entry(host.clone()) {
            StdEntry::Occupied(a) => a.into_mut(),
            StdEntry::Vacant(a) => a.insert(ServerWebConf {
                web_conf: WebConf::default(),
                web_conf_when: None,
            }),
        };

        let trigger = match &conf.web_conf_when {
            Some(a) if a.elapsed().as_millis() > 4000u128 => true,
            None => true,
            _ => {
                return Ok(conf.web_conf.clone());
            }
        };

        #[allow(unused_assignments, unused_variables)]
        if trigger {
            let mut is_done = false;

            if let Some(www_path) = self.www_path.clone() {
                let abs_path = PathBuf::from(www_path).join(host.as_str()).join(WEB_CONF_FILES_CONF);
                trace!("searching locally: {}", abs_path.to_string_lossy());
                if let Ok(data) = tokio::fs::read(abs_path).await {
                    let data = String::from_utf8_lossy(&data[..]);
                    conf.web_conf_when = Some(Instant::now());
                    conf.web_conf =
                        serde_yaml::from_str::<WebConf>(&data).map_err(|err| {
                            WebServerError::from_kind(WebServerErrorKind::BadConfiguration(
                                err.to_string(),
                            ))
                        })?;
                    is_done = true;
                }
            }

            #[cfg(feature = "dfs")]
            if is_done == false {
                let key = ChainKey::from(format!("{}/www", host));
                conf.web_conf_when = Some(Instant::now());
                conf.web_conf = match self
                    .repo
                    .get_file(&key, host.as_str(), WEB_CONF_FILES_CONF)
                    .await
                    .ok()
                    .flatten()
                {
                    Some(data) => {
                        let data = String::from_utf8_lossy(&data[..]);
                        serde_yaml::from_str::<WebConf>(&data).map_err(|err| {
                            WebServerError::from_kind(WebServerErrorKind::BadConfiguration(
                                err.to_string(),
                            ))
                        })?
                    }
                    None => {
                        let mut ret = WebConf::default();
                        ret.default_page = Some("index.html".to_string());
                        ret.force_https = true;

                        if let Some(ret_str) = serde_yaml::to_string(&ret).ok() {
                            let err = self
                                .repo
                                .set_file(&key, host.as_str(), WEB_CONF_FILES_CONF, ret_str.as_bytes())
                                .await;
                            if let Err(err) = err {
                                info!("failed to save default web.yaml - {}", err);
                            }
                        }
                        ret
                    }
                };
                is_done = true;
            }
        }

        match serde_yaml::to_string(&conf.web_conf) {
            Ok(conf) => trace!("web-conf: {}", conf),
            Err(err) => trace!("web-conf-err: {}", err),
        };
        Ok(conf.web_conf.clone())
    }

    pub(crate) async fn force_https(
        &self,
        req: Request<Body>,
    ) -> Result<Response<Body>, WebServerError> {
        let host = match req.uri().authority() {
            Some(a) => a.to_string(),
            None => match req.headers().get("Host") {
                Some(a) => a.to_str()?.to_string(),
                None => {
                    bail!(WebServerErrorKind::BadRequest(
                        "unknown host address needed for redirect to https".to_string()
                    ))
                }
            },
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
        }
        .to_string();

        self.process_redirect(uri.as_str()).await
    }

    pub(crate) async fn process_redirect_host(
        &self,
        req: Request<Body>,
        #[allow(unused_variables)]
        listen: &ServerListen,
        redirect: &str,
    ) -> Result<Response<Body>, WebServerError> {
        let mut uri = http::Uri::builder().authority(redirect);
        if let Some(scheme) = req.uri().scheme() {
            uri = uri.scheme(scheme.clone());
        } else {
            uri = uri.scheme("http");
        }
        if listen.tls {
            uri = uri.scheme("https");
        }
        if let Some(path_and_query) = req.uri().path_and_query() {
            uri = uri.path_and_query(path_and_query.clone());
        }
        let uri = match uri.build() {
            Ok(uri) => uri,
            Err(err) => {
                bail!(WebServerErrorKind::BadRequest(err.to_string()))
            }
        }
        .to_string();

        self.process_redirect(uri.as_str()).await
    }

    pub(crate) async fn process_redirect(
        &self,
        uri: &str,
    ) -> Result<Response<Body>, WebServerError> {
        let mut resp = Response::new(Body::from(crate::helper::redirect_body(uri)));
        resp.headers_mut()
            .append("Location", HeaderValue::from_str(uri)?);
        *resp.status_mut() = StatusCode::PERMANENT_REDIRECT;
        return Ok(resp);
    }

    pub(crate) fn sanitize<'a>(&self, mut path: &'a str) -> Result<&'a str, WebServerError> {
        while path.starts_with("/") {
            path = &path[1..];
        }
        if path.contains("~") {
            bail!(WebServerErrorKind::BadRequest(
                "Accessing home directories is forbidden".to_string()
            ));
        }
        if path.contains("..") {
            bail!(WebServerErrorKind::BadRequest(
                "Accessing parent directories is forbidden".to_string()
            ));
        }
        if path.starts_with(WEB_CONF_FILES) {
            bail!(WebServerErrorKind::BadRequest(
                "Accessing configuration files is forbidden".to_string()
            ));
        }

        // If it has parameters passed to the web server we ignore them
        path = if let Some((left, _right)) = path.split_once("?") {
            left
        } else {
            path
        };

        Ok(path)
    }

    #[allow(unused_variables)]
    pub(crate) async fn process_get(
        &self,
        host: &str,
        mut path: &str,
        is_head: bool,
        conf: &WebConf,
    ) -> Result<Option<Response<Body>>, WebServerError> {
        path = self.sanitize(path)?;

        let mut data = None;
        if let Some(www_path) = self.www_path.clone() {
            let abs_path = PathBuf::from(www_path).join(host).join(path);
            trace!("searching locally: {}", abs_path.to_string_lossy());

            // Attempt to open the file
            if let Ok(d) = tokio::fs::read(abs_path).await {
                data = Some(Bytes::from(d));
            }
        }

        #[cfg(feature = "dfs")]
        if data.is_none() {
            let key = ChainKey::from(format!("{}/www", host));
            trace!("perf-checkpoint: get_file (path={})", path);
            if let Some(d) = self.repo.get_file(&key, host, path).await? {
                data = Some(d);
            }
        }

        if let Some(data) = data {
            let len_str = data.len().to_string();
            trace!("perf-checkpoint: got_file (data_len={})", len_str);

            let mut resp = if is_head {
                Response::new(Body::empty())
            } else {
                Response::new(Body::from(data))
            };
            resp.headers_mut()
                .append("Content-Length", HeaderValue::from_str(len_str.as_str())?);
            self.apply_mime(path, &mut resp)?;
            if conf.coop {
                resp.headers_mut().append(
                    "Cross-Origin-Embedder-Policy",
                    HeaderValue::from_str("require-corp")?,
                );
                resp.headers_mut().append(
                    "Cross-Origin-Opener-Policy",
                    HeaderValue::from_str("same-origin")?,
                );
            }
            *resp.status_mut() = StatusCode::OK;
            return Ok(Some(resp));
        }

        Ok(None)
    }

    #[allow(dead_code)]
    pub(crate) fn apply_mime(
        &self,
        path: &str,
        resp: &mut Response<Body>,
    ) -> Result<(), WebServerError> {
        if let Some(ext) = path.split(".").collect::<Vec<_>>().into_iter().rev().next() {
            let ext = ext.to_string();
            if let Some(mime) = self.mime.get(&ext) {
                resp.headers_mut()
                    .append("Content-Type", HeaderValue::from_str(mime.as_str())?);
            }
        }
        Ok(())
    }

    fn init_mime() -> HashMap<String, String> {
        let mut ret = HashMap::default();
        ret.insert("aac".to_string(), "audio/aac".to_string());
        ret.insert("abw".to_string(), "application/x-abiword".to_string());
        ret.insert("arc".to_string(), "application/x-freearc".to_string());
        ret.insert("avi".to_string(), "video/x-msvideo".to_string());
        ret.insert(
            "azw".to_string(),
            "application/vnd.amazon.ebook".to_string(),
        );
        ret.insert("bin".to_string(), "application/octet-stream".to_string());
        ret.insert("bmp".to_string(), "image/bmp".to_string());
        ret.insert("bz".to_string(), "application/x-bzip".to_string());
        ret.insert("bz2".to_string(), "application/x-bzip2".to_string());
        ret.insert("cda".to_string(), "application/x-cdf".to_string());
        ret.insert("csh".to_string(), "application/x-csh".to_string());
        ret.insert("css".to_string(), "text/css".to_string());
        ret.insert("csv".to_string(), "text/csv".to_string());
        ret.insert("doc".to_string(), "application/msword".to_string());
        ret.insert(
            "docx".to_string(),
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string(),
        );
        ret.insert(
            "eot".to_string(),
            "application/vnd.ms-fontobject".to_string(),
        );
        ret.insert("epub".to_string(), "application/epub+zip".to_string());
        ret.insert("gz".to_string(), "application/gzip".to_string());
        ret.insert("gif".to_string(), "image/gif".to_string());
        ret.insert("htm".to_string(), "text/html".to_string());
        ret.insert("html".to_string(), "text/html".to_string());
        ret.insert("ico".to_string(), "image/vnd.microsoft.icon".to_string());
        ret.insert("ics".to_string(), "text/calendar".to_string());
        ret.insert("jar".to_string(), "application/java-archive".to_string());
        ret.insert("jpeg".to_string(), "image/jpeg".to_string());
        ret.insert("jpg".to_string(), "image/jpeg".to_string());
        ret.insert("js".to_string(), "text/javascript".to_string());
        ret.insert("json".to_string(), "application/json".to_string());
        ret.insert("jsonld".to_string(), "application/ld+json".to_string());
        ret.insert("mid".to_string(), "audio/midi".to_string());
        ret.insert("midi".to_string(), "audio/midi".to_string());
        ret.insert("mjs".to_string(), "text/javascript".to_string());
        ret.insert("mp3".to_string(), "audio/mpeg".to_string());
        ret.insert("mp4".to_string(), "video/mp4".to_string());
        ret.insert("mpeg".to_string(), "video/mpeg".to_string());
        ret.insert(
            "mpkg".to_string(),
            "application/vnd.apple.installer+xml".to_string(),
        );
        ret.insert(
            "odp".to_string(),
            "application/vnd.oasis.opendocument.presentation".to_string(),
        );
        ret.insert(
            "ods".to_string(),
            "application/vnd.oasis.opendocument.spreadsheet".to_string(),
        );
        ret.insert(
            "odt".to_string(),
            "application/vnd.oasis.opendocument.text".to_string(),
        );
        ret.insert("oga".to_string(), "audio/ogg".to_string());
        ret.insert("ogv".to_string(), "video/ogg".to_string());
        ret.insert("ogx".to_string(), "application/ogg".to_string());
        ret.insert("opus".to_string(), "audio/opus".to_string());
        ret.insert("otf".to_string(), "font/otf".to_string());
        ret.insert("png".to_string(), "image/png".to_string());
        ret.insert("pdf".to_string(), "application/pdf".to_string());
        ret.insert("php".to_string(), "application/x-httpd-php".to_string());
        ret.insert(
            "ppt".to_string(),
            "application/vnd.ms-powerpoint".to_string(),
        );
        ret.insert(
            "pptx".to_string(),
            "application/vnd.openxmlformats-officedocument.presentationml.presentation".to_string(),
        );
        ret.insert("rar".to_string(), "application/vnd.rar".to_string());
        ret.insert("rtf".to_string(), "application/rtf".to_string());
        ret.insert("sh".to_string(), "application/x-sh".to_string());
        ret.insert("svg".to_string(), "image/svg+xml".to_string());
        ret.insert(
            "swf".to_string(),
            "application/x-shockwave-flash".to_string(),
        );
        ret.insert("tar".to_string(), "application/x-tar".to_string());
        ret.insert("tif".to_string(), "image/tiff".to_string());
        ret.insert("tiff".to_string(), "image/tiff".to_string());
        ret.insert("ts".to_string(), "video/mp2t".to_string());
        ret.insert("ttf".to_string(), "font/ttf".to_string());
        ret.insert("txt".to_string(), "text/plain".to_string());
        ret.insert("vsd".to_string(), "application/vnd.visio".to_string());
        ret.insert("wav".to_string(), "audio/wav".to_string());
        ret.insert("wasm".to_string(), "application/wasm".to_string());
        ret.insert("weba".to_string(), "audio/webm".to_string());
        ret.insert("webm".to_string(), "video/webm".to_string());
        ret.insert("webp".to_string(), "image/webp".to_string());
        ret.insert("woff".to_string(), "font/woff".to_string());
        ret.insert("woff2".to_string(), "font/woff2".to_string());
        ret.insert("xhtml".to_string(), "application/xhtml+xml".to_string());
        ret.insert("xls".to_string(), "application/vnd.ms-excel".to_string());
        ret.insert(
            "xlsx".to_string(),
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string(),
        );
        ret.insert("xml".to_string(), "application/xml".to_string());
        ret.insert(
            "xul".to_string(),
            "application/vnd.mozilla.xul+xml".to_string(),
        );
        ret.insert("zip".to_string(), "application/zip".to_string());
        ret.insert("7z".to_string(), "application/x-7z-compressed".to_string());
        ret
    }

    pub(crate) async fn process_get_with_default(
        &self,
        host: &str,
        mut path: &str,
        is_head: bool,
        conf: &WebConf,
    ) -> Result<Response<Body>, WebServerError> {
        path = self.sanitize(path)?;

        // Attempt to get the file
        trace!("perf-checkpoint: process_get");
        match self.process_get(host, path, is_head, conf).await? {
            Some(a) => {
                return Ok(a);
            }
            None => {
                // Otherwise we attempt to get a default file
                let default_page = conf.default_page.as_ref();
                if let Some(default_page) = default_page {
                    let path = if path.ends_with("/") == false {
                        format!("{}/{}", path, default_page)
                    } else {
                        format!("{}{}", path, default_page)
                    };
                    if let Some(ret) = self.process_get(host, path.as_str(), is_head, conf).await? {
                        return Ok(ret);
                    }
                }
            }
        }

        trace!("perf-checkpoint: response from data");
        let data = format!("File Not Found - {}\n", path);
        let mut resp = Response::new(Body::from(data));
        *resp.status_mut() = StatusCode::NOT_FOUND;
        Ok(resp)
    }

    pub(crate) async fn process(
        &self,
        req: Request<Body>,
        sock_addr: SocketAddr,
        listen: &ServerListen,
    ) -> Result<Response<Body>, WebServerError> {
        trace!("req: {:?}", req);

        if hyper_tungstenite::is_upgrade_request(&req) {
            trace!("perf-checkpoint: hyper upgrade request");
            return self.process_upgrade(req, sock_addr).await;
        }

        let uri = req.uri().clone();
        let method = req.method().clone();

        if method == Method::POST || method == Method::PUT {
            trace!("perf-checkpoint: put/post");

            if let Some(callback) = &self.callback {
                let headers = req.headers().clone();
                if let Some(body) = hyper::body::to_bytes(req.into_body()).await.ok() {
                    let body = body.to_vec();
                    let (resp, status) = match method {
                        Method::POST => callback.post_request(body, sock_addr, uri, headers).await,
                        Method::PUT => callback.put_request(body, sock_addr, uri, headers).await,
                        _ => (Vec::new(), StatusCode::BAD_REQUEST)
                    };
                    let mut resp = Response::new(Body::from(resp));
                    *resp.status_mut() = status;
                    trace!("res: status={}", resp.status().as_u16());
                    return Ok(resp);
                } else {
                    let status = StatusCode::BAD_REQUEST;
                    let err = status.as_str().to_string();
                    let mut resp = Response::new(Body::from(err));
                    *resp.status_mut() = status;
                    trace!("res: status={}", resp.status().as_u16());
                    return Ok(resp);
                }
            }

            trace!("perf-checkpoint: finished put/post");
        }

        let is_head = method == Method::HEAD;
        let host = self.get_host(&req)?;
        let conf = self.get_conf(host.as_str()).await?;

        let ret = self.process_internal(req, listen, &conf).await;
        match ret {
            Ok(a) => {
                info!("http peer={} method={} path={} - {}", sock_addr, method, uri, a.status());
                Ok(a)
            },
            Err(err) => {
                info!("http peer={} method={} path={} err={}", sock_addr, method, uri, err);
                let page = conf
                    .status_pages
                    .get(&err.status_code().as_u16())
                    .map(|a| a.clone());
                if let Some(page) = page {
                    trace!("perf-checkpoint: load error page");
                    if let Some(ret) = self
                        .process_get(host.as_str(), page.as_str(), is_head, &conf)
                        .await?
                    {
                        return Ok(ret);
                    }
                }
                return Err(err);
            }
        }
    }

    pub(crate) async fn process_upgrade(
        &self,
        req: Request<Body>,
        sock_addr: SocketAddr,
    ) -> Result<Response<Body>, WebServerError> {
        if let Some(callback) = &self.callback {
            let uri = req.uri().clone();
            let headers = req.headers().clone();
            let callback = Arc::clone(callback);
            let (response, websocket) = hyper_tungstenite::upgrade(req, None)?;
            tokio::task::spawn(async move {
                match websocket.await {
                    Ok(websocket) => {
                        trace!("perf-checkpoint: begin callback.web_socket");
                        let ret = callback.web_socket(websocket, sock_addr, Some(uri), Some(headers)).await;
                        trace!("perf-checkpoint: finish callback.web_socket");
                        if let Err(err) = ret {
                            error!("web socket failed(1) - {}", err);
                        }
                    }
                    Err(err) => {
                        error!("web socket failed(2) - {}", err);
                    }
                }
            });
            Ok(response)
        } else {
            Err(WebServerErrorKind::BadRequest("websockets are not supported".to_string()).into())
        }
    }

    #[cfg(feature = "cors")]
    pub(crate) async fn process_cors(
        &self,
        req: Request<Body>,
        _listen: &ServerListen,
        conf: &WebConf,
        target: String
    ) -> Result<Response<Body>, StatusCode> {
        let mut uri = format!("https://{}", target);
        if let Some(query) = req.uri().query() {
            uri += "?";
            uri += query;
        }
        if let Ok(uri) = http::uri::Uri::from_str(uri.as_str())
        {
            // Check if its allowed
            if conf.cors_proxy
                .iter()
                .map(|cors| Some(cors.as_str()))
                .any(|cors| cors == uri.authority().map(|a| a.as_str()))
            {
                let method = req.method().clone();
                let client = reqwest::ClientBuilder::default().build().map_err(|err| {
                    debug!("failed to build reqwest client - {}", err);
                    StatusCode::BAD_REQUEST
                })?;

                let mut builder = client.request(method, uri.to_string().as_str());
                for (header, val) in req.headers() {
                    builder = builder.header(header, val);
                }
                let body = hyper::body::to_bytes(req.into_body()).await
                    .map_err(|err| {
                    debug!("failed to build reqwest body - {}", err);
                    StatusCode::BAD_REQUEST
                })?;
                builder = builder.body(reqwest::Body::from(body));

                let request = builder.build().map_err(|err| {
                    debug!("failed to convert request (url={}) - {}", uri, err);
                    StatusCode::BAD_REQUEST
                })?;

                let response = client.execute(request).await.map_err(|err| {
                    debug!("failed to execute reqest - {}", err);
                    StatusCode::BAD_REQUEST
                })?;

                let status = response.status();
                let headers = response.headers().clone();
                let data = response.bytes().await.map_err(|err| {
                    debug!("failed to read response bytes - {}", err);
                    StatusCode::BAD_REQUEST
                })?;
                let data = data.to_vec();

                let mut resp = Response::new(Body::from(data));
                for (header, val) in headers {
                    if let Some(header) = header {
                        resp.headers_mut()
                            .append(header, val);
                    }
                }
                if resp.headers().contains_key(http::header::ACCESS_CONTROL_ALLOW_ORIGIN) == false {
                    resp.headers_mut()
                        .append(http::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
                }
                if resp.headers().contains_key(http::header::ACCESS_CONTROL_ALLOW_METHODS) == false {
                    resp.headers_mut()
                        .append(http::header::ACCESS_CONTROL_ALLOW_METHODS, "*".parse().unwrap());
                }
                if resp.headers().contains_key(http::header::ACCESS_CONTROL_ALLOW_HEADERS) == false {
                    resp.headers_mut()
                        .append(http::header::ACCESS_CONTROL_ALLOW_HEADERS, "*".parse().unwrap());
                }
                if resp.headers().contains_key(http::header::ACCESS_CONTROL_MAX_AGE) == false {
                    resp.headers_mut()
                        .append(http::header::ACCESS_CONTROL_MAX_AGE, "86400".parse().unwrap());
                }
                *resp.status_mut() = status;
                return Ok(resp);
            } else {
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
            
        return Err(StatusCode::BAD_REQUEST);
    }

    pub(crate) async fn process_internal(
        &self,
        req: Request<Body>,
        listen: &ServerListen,
        conf: &WebConf,
    ) -> Result<Response<Body>, WebServerError> {
        if let Some(redirect) = conf.redirect.as_ref() {
            trace!("perf-checkpoint: redirect host");
            return self.process_redirect_host(req, listen, &redirect).await;
        }

        if conf.force_https && listen.tls == false {
            trace!("perf-checkpoint: force_https");
            return self.force_https(req).await;
        }

        #[cfg(feature = "cors")]
        {
            let mut cors_proxy = req.uri().path().split("https://");
            cors_proxy.next();
            if let Some(next) = cors_proxy.next() {
                trace!("perf-checkpoint: cors proxy");
                let next = next.to_string();
                return Ok(self.process_cors(req, listen, conf, next).await
                    .unwrap_or_else(|code| {
                        let mut resp = Response::new(Body::from(code.as_str().to_string()));
                        *resp.status_mut() = code;
                        resp
                    }));
            }
        }

        let host = self.get_host(&req)?;
        let is_head = req.method() == Method::HEAD || req.method() == Method::OPTIONS;

        let path = req.uri().path();
        match req.method() {
            &Method::OPTIONS | &Method::HEAD | &Method::GET => {
                trace!("perf-checkpoint: options/head/get");
                self.sanitize(path)?;
                self.process_get_with_default(host.as_str(), path, is_head, conf)
                    .await
            }
            _ => {
                trace!("perf-checkpoint: method_not_allowed");
                let mut resp = Response::new(Body::from(StatusCode::METHOD_NOT_ALLOWED.as_str()));
                *resp.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
                Ok(resp)
            }
        }
    }
}

#[allow(dead_code)]
struct SessionFactory
{
    #[cfg(feature = "ate")]
    master_key: Option<EncryptKey>,
    #[cfg(feature = "dfs")]
    registry: Arc<Registry>,
    #[cfg(feature = "wasmer-auth")]
    auth_url: url::Url,
}

#[cfg(feature = "dfs")]
#[async_trait]
impl RepositorySessionFactory
for SessionFactory
{
    async fn create(&self, sni: String, _key: ChainKey) -> Result<AteSessionType, AteError>
    {
        // Create the session
        let key_entropy = format!("{}:{}", "web-read", sni);
        let key_entropy = AteHash::from_bytes(key_entropy.as_bytes());
        let mut session = AteSessionUser::default();

        // If we have a master key then add it
        if let Some(master_key) = &self.master_key {
            let read_key = AuthService::compute_super_key_from_hash(master_key, &key_entropy);
            session.add_user_read_key(&read_key);
        }

        // Now attempt to gather permissions to the chain
        let registry = self.registry.clone();
        let auth_url = self.auth_url.clone();
        
        // Now we gather the rights to the particular domain this website is running under
        Ok(AteSessionType::Group(gather_command(
            &registry,
            sni,
            AteSessionInner::User(session),
            auth_url.clone(),
        )
        .await?))
    }
}