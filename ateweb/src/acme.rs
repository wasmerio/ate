#[allow(unused_imports, dead_code)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use tokio_rustls::webpki::DNSName;
use tokio_rustls::webpki::DNSNameRef;
use rustls::sign::any_supported_type;
use rustls::Certificate as RustlsCertificate;
use rustls::PrivateKey as RustlsPrivateKey;
use rustls::ResolvesServerCert;
use rustls::ClientHello;
use rustls::sign::CertifiedKey;
use std::sync::Arc;
use fxhash::FxHashMap;
use ate::engine::TaskEngine;
use tokio::sync::mpsc;
use ate::prelude::*;
use std::ops::Deref;
use tokio::select;
use tokio::sync::Mutex;
use parking_lot::RwLock;

use crate::model::*;

pub struct Acme
{
    pub dio: Arc<DioMut>,
    pub bus: Mutex<Bus<CertificateKey>>,
    pub store: DaoMut<CertificateStore>,
    pub certs: RwLock<FxHashMap<DNSName, CertifiedKey>>,
    pub touch_tx: mpsc::Sender<String>,
}

impl Acme
{
    pub async fn new(dio: Arc<DioMut>) -> Result<Arc<Acme>, AteError>
    {
        let store = match dio.try_load(&PrimaryKey::from(CERT_STORE_ID)).await? {
            Some(a) => a,
            None => {
                let ret = dio.store_with_key(CertificateStore::default(), PrimaryKey::from(CERT_STORE_ID))?;
                dio.commit().await?;
                ret
            }
        };
        let bus = store.certs.bus().await?;

        let (tx, rx) = mpsc::channel(5000usize);
        let mut ret = Acme {
            dio,
            bus: Mutex::new(bus),
            store,
            certs: RwLock::new(FxHashMap::default()),
            touch_tx: tx,
        };

        ret.init().await?;
        
        let ret = Arc::new(ret);
        TaskEngine::spawn(Arc::clone(&ret).run(rx));

        Ok(ret)
    }
}

impl Acme
{
    async fn run(self: Arc<Acme>, mut touch_rx: mpsc::Receiver<String>) {
        loop {
            let mut guard = self.bus.lock().await;
            select! {
                a = touch_rx.recv() => {
                    drop(guard);
                    let sni = match a {
                        Some(a) => a,
                        None => { break; }
                    };
                    self.touch(sni).await;
                },
                a = guard.recv() => {
                    match a {
                        Ok(a) => {
                            if let Ok(Some((sni, cert))) = self.load(a.deref()).await {
                                let mut guard = self.certs.write();
                                guard.insert(sni, cert);
                            }
                        },
                        Err(err) => {
                            error!("failed to process received certificate - {}", err);
                        }
                    };
                }
            }            
        }
    }

    async fn touch(&self, _sni: String) {

    }

    async fn load(&self, cert: &CertificateKey) -> Result<Option<(DNSName, CertifiedKey)>, AteError> {
        let sni = match DNSNameRef::try_from_ascii_str(cert.domain.as_str()) {
            Ok(a) => a.to_owned(),
            Err(err) => {
                warn!("failed to load cert ({}) - {}", cert.domain, err);
                return Ok(None);
            }
        };
        let pk = match any_supported_type(&RustlsPrivateKey(cert.pk.clone())) {
            Ok(a) => a,
            Err(()) => {
                warn!("failed to load cert ({}) - unsupported signing key", cert.domain);
                return Ok(None);
            }
        };
        let certs = cert.certs.iter()
            .map(|a| RustlsCertificate(a.data.clone()))
            .collect::<Vec<_>>();
        let cert_key = CertifiedKey::new(certs, Arc::new(pk));
        Ok(Some(
            (sni, cert_key)
        ))
    }

    async fn init(&mut self) -> Result<(), AteError> {
        for cert in self.store.certs.iter().await? {
            if let Some((sni, cert)) = self.load(cert.deref()).await? {
                let mut guard = self.certs.write();
                guard.insert(sni, cert);
            }
        }
        Ok(())
    }
}

impl ResolvesServerCert
for Acme
{
    fn resolve(&self, client_hello: ClientHello) -> Option<CertifiedKey> {
        if let Some(sni) = client_hello.server_name() {
            let sni = sni.to_owned();

            let guard = self.certs.read();
            if let Some(cert) = guard.get(&sni)  {
                trace!("tls_hello: cert_hit={:?}", sni);
                return Some(cert.clone());
            }

            let sni: &str = sni.as_ref().clone().into();
            let sni = sni.to_string();
            trace!("tls_hello: cert_miss={:?}", sni);
            let _ = self.touch_tx.blocking_send(sni);
        } else {
            debug!("rejected connection (SNI was missing)");
        }
        None
    }
}