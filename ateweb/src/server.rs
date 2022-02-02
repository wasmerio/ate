use async_trait::async_trait;
use error_chain::bail;
use fxhash::FxHashMap;
use std::collections::hash_map::Entry as StdEntry;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::Weak;
use std::time::Duration;
use std::time::Instant;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, instrument, span, trace, warn, event, Level};
use ate_auth::service::AuthService;
use ate_auth::cmd::gather_command;

use hyper;
use hyper::header::HeaderValue;
use hyper::service::{make_service_fn, service_fn};
use hyper::upgrade::Upgraded;
use hyper::Body;
use hyper::Method;
use hyper::Request;
use hyper::Response;
use hyper::StatusCode;
use hyper_tungstenite::WebSocketStream;

use ate::prelude::*;
use ate_files::prelude::*;
use ate_files::repo::*;

use crate::model::WebConf;

use super::acceptor::*;
use super::acme::AcmeResolver;
use super::builder::*;
use super::conf::*;
use super::error::WebServerError;
use super::error::WebServerErrorKind;
use super::model::*;
use super::stream::*;

pub struct ServerWebConf {
    web_conf: WebConf,
    web_conf_when: Option<Instant>,
}

#[async_trait]
pub trait ServerCallback: Send + Sync {
    async fn web_socket(
        &self,
        _ws: WebSocketStream<Upgraded>,
        _sock_addr: SocketAddr,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

pub struct Server {
    repo: Arc<Repository>,
    web_conf: Mutex<FxHashMap<String, ServerWebConf>>,
    server_conf: ServerConf,
    callback: Option<Arc<dyn ServerCallback>>,
    mime: FxHashMap<String, String>,
}

async fn process(
    server: Arc<Server>,
    listen: Arc<ServerListen>,
    req: Request<Body>,
    sock_addr: SocketAddr,
) -> Result<Response<Body>, hyper::Error> {
    let path = req.uri().path().to_string();
    match server.process(req, sock_addr, listen.deref()).await {
        Ok(resp) => {
            trace!("res: status={}", resp.status().as_u16());
            Ok(resp)
        }
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
    pub(crate) async fn new(builder: ServerBuilder) -> Result<Arc<Server>, AteError> {
        let registry = Arc::new(Registry::new(&builder.conf.cfg_ate).await);

        let session_factory = SessionFactory {
            auth_url: builder.auth_url.clone(),
            registry: registry.clone(),
            master_key: builder.web_master_key.clone(),
        };

        let repo = Repository::new(
            &registry,
            builder.remote.clone(),
            builder.auth_url.clone(),
            Box::new(session_factory),
            builder.conf.ttl,
        )
        .await?;

        Ok(Arc::new(Server {
            repo,
            web_conf: Mutex::new(FxHashMap::default()),
            server_conf: builder.conf,
            callback: builder.callback,
            mime: Server::init_mime(),
        }))
    }

    pub async fn run(self: &Arc<Self>) -> Result<(), Box<dyn std::error::Error>> {
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
                    async move {
                        Ok::<_, Infallible>(service_fn(move |req| {
                            process(server.clone(), listen.clone(), req, addr)
                        }))
                    }
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

    async fn house_keeping(&self) {
        self.repo.house_keeping().await;
    }

    pub(crate) fn get_host(&self, req: &Request<Body>) -> Result<String, WebServerError> {
        if let Some(host) = req.uri().host() {
            return Ok(host.to_string());
        }
        match req.headers().get("Host") {
            Some(a) => Ok(a.to_str()?.to_string()),
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
        if trigger {
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
        listen: &ServerListen,
        redirect: &str,
    ) -> Result<Response<Body>, WebServerError> {
        let mut uri = http::Uri::builder().authority(redirect);
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

    pub(crate) fn sanitize(&self, mut path: &str) -> Result<(), WebServerError> {
        while path.starts_with("/") {
            path = &path[1..];
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
        Ok(())
    }

    pub(crate) async fn process_get(
        &self,
        host: &str,
        path: &str,
        is_head: bool,
        conf: &WebConf,
    ) -> Result<Option<Response<Body>>, WebServerError> {
        self.sanitize(path)?;
        let key = ChainKey::from(format!("{}/www", host));
        if let Some(data) = self.repo.get_file(&key, host, path).await? {
            let len_str = data.len().to_string();

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
            Ok(Some(resp))
        } else {
            Ok(None)
        }
    }

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

    fn init_mime() -> FxHashMap<String, String> {
        let mut ret = FxHashMap::default();
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
        path: &str,
        is_head: bool,
        conf: &WebConf,
    ) -> Result<Response<Body>, WebServerError> {
        self.sanitize(path)?;

        // If it has parameters passed to the web server we ignore them
        let path = if let Some((left, _right)) = path.split_once("?") {
            left.to_string()
        } else {
            path.to_string()
        };

        // Attempt to get the file
        match self.process_get(host, path.as_str(), is_head, conf).await? {
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
            return self.process_upgrade(req, sock_addr).await;
        }

        let uri = req.uri().clone();
        let method = req.method().clone();
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
            let callback = Arc::clone(callback);
            let (response, websocket) = hyper_tungstenite::upgrade(req, None)?;
            TaskEngine::spawn(async move {
                match websocket.await {
                    Ok(websocket) => {
                        let ret = callback.web_socket(websocket, sock_addr).await;
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

    pub(crate) async fn process_internal(
        &self,
        req: Request<Body>,
        listen: &ServerListen,
        conf: &WebConf,
    ) -> Result<Response<Body>, WebServerError> {
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
                self.process_get_with_default(host.as_str(), path, is_head, conf)
                    .await
            }
            _ => {
                let mut resp = Response::new(Body::from(StatusCode::METHOD_NOT_ALLOWED.as_str()));
                *resp.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
                Ok(resp)
            }
        }
    }
}

struct SessionFactory
{
    master_key: Option<EncryptKey>,
    registry: Arc<Registry>,
    auth_url: url::Url,
}

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