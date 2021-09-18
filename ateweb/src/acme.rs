#[allow(unused_imports, dead_code)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use rustls::Certificate as RustlsCertificate;
use rustls::ResolvesServerCert;
use rustls::ClientHello;
use rustls::PrivateKey;
use rustls::sign::any_ecdsa_type;
use rustls::sign::CertifiedKey;
use std::sync::Arc;
use ate::engine::TaskEngine;
use tokio::sync::mpsc;
use ate::prelude::*;
use tokio::select;
use parking_lot::RwLock;
use rustls_acme::acme::ACME_TLS_ALPN_NAME;
use ttl_cache::TtlCache;
use bytes::Bytes;

use crate::repo::*;
use crate::model::*;

pub struct Acme
{
    pub repo: Arc<Repository>,
    pub certs: RwLock<TtlCache<String, CertifiedKey>>,
    pub auths: RwLock<TtlCache<String, CertifiedKey>>,
    pub touch_tx: mpsc::Sender<String>,
}

impl Acme
{
    pub async fn new(repo: &Arc<Repository>) -> Result<Arc<Acme>, AteError>
    {
        let (tx, rx) = mpsc::channel(5000usize);
        let ret = Acme {
            repo: Arc::clone(repo),
            certs: RwLock::new(TtlCache::new(usize::MAX)),
            auths: RwLock::new(TtlCache::new(usize::MAX)),
            touch_tx: tx,
        };
        
        let ret = Arc::new(ret);
        {
            let ret = Arc::clone(&ret);
            TaskEngine::spawn(ret.run(rx));
        }
        Ok(ret)
    }
}

impl Acme
{
    async fn run(self: Arc<Acme>, mut touch_rx: mpsc::Receiver<String>) {
        loop {
            select! {
                a = touch_rx.recv() => {
                    let sni = match a {
                        Some(a) => a,
                        None => { break; }
                    };
                    let ret = self.touch(sni).await;
                    if let Err(err) = ret {
                        warn!("acme-failed: {}", err);
                    }
                },
            }            
        }
    }

    async fn process_cert(&self, sni: &str, cert: Bytes) -> Result<(), Box<dyn std::error::Error>>
    {
        let mut pems = pem::parse_many(&cert[..]);
        if pems.len() < 2 {
            error!(
                "expected 2 or more pem in {}, got: {}",
                sni,
                pems.len()
            );
            return Ok(());
        }
        let pk = match any_ecdsa_type(&PrivateKey(pems.remove(0).contents)) {
            Ok(pk) => pk,
            Err(_) => {
                error!("{} does not contain an ecdsa private key", sni);
                return Ok(());
            }
        };
        let cert_chain = pems
            .into_iter()
            .map(|p| RustlsCertificate(p.contents))
            .collect();
        let cert_key = CertifiedKey::new(cert_chain, Arc::new(pk));

        let mut guard = self.certs.write();
        guard.insert(sni.to_string(), cert_key, self.repo.ttl);

        Ok(())
    }

    async fn touch(&self, sni: String) -> Result<(), Box<dyn std::error::Error>>
    {
        {
            let guard = self.certs.read();
            if guard.contains_key(&sni) {
                return Ok(())
            }
        }

        let cert = self.repo.get_file(sni.as_str(), WEB_CONF_FILES_CERT).await?;
        if let Some(cert) = cert {
            self.process_cert(sni.as_str(), cert).await?;
        }

        Ok(())

        // Check if we are in a global renewal freeze

        /*
        // Order the certificate
        let mut err_cnt = 0usize;
        loop {
            let d = self.duration_until_renewal_attempt(err_cnt);
            if d.as_secs() != 0 {
                debug!("next renewal attempt in {}s", d.as_secs());
                sleep(d).await;
            }
            match self
                .order(&directory_url, &domains, &cache_dir, &file_name)
                .await
            {
                Ok(_) => {
                    debug!("successfully ordered certificate");
                    err_cnt = 0;
                }
                Err(err) => {
                    warn!("ordering certificate failed: {}", err);
                    err_cnt += 1;
                }
            };
        }
        */

        // Get the challenge 
    }

    /*
    fn duration_until_renewal_attempt(&self, err_cnt: usize) -> Duration {
        let valid_until = match self.cert_key.lock().unwrap().clone() {
            None => 0,
            Some(cert_key) => match cert_key.cert.first() {
                Some(cert) => match parse_x509_certificate(cert.0.as_slice()) {
                    Ok((_, cert)) => cert.validity().not_after.timestamp(),
                    Err(err) => {
                        warn!("could not parse certificate: {}", err);
                        0
                    }
                },
                None => 0,
            },
        };
        let valid_secs = (valid_until - Utc::now().timestamp()).max(0);
        let wait_secs = Duration::from_secs(valid_secs as u64 / 2);
        match err_cnt {
            0 => wait_secs,
            err_cnt => wait_secs.max(Duration::from_secs(1 << err_cnt)),
        }
    }

    async fn order<P: AsRef<Path>>(
        &self,
        directory_url: impl AsRef<str>,
        domains: &Vec<String>,
        cache_dir: &Option<P>,
        file_name: &str,
    ) -> Result<(), OrderError> {
        let mut params = CertificateParams::new(domains.clone());
        params.distinguished_name = DistinguishedName::new();
        params.alg = &PKCS_ECDSA_P256_SHA256;
        let cert = rcgen::Certificate::from_params(params)?;
        let pk = any_ecdsa_type(&PrivateKey(cert.serialize_private_key_der())).unwrap();
        let directory = Directory::discover(directory_url).await?;
        let account = Account::load_or_create(directory, cache_dir.as_ref(), &self.contact).await?;
        let mut order = account.new_order(domains.clone()).await?;
        loop {
            order = match order {
                Order::Pending {
                    authorizations,
                    finalize,
                } => {
                    let auth_futures = authorizations
                        .iter()
                        .map(|url| self.authorize(&account, url));
                    try_join_all(auth_futures).await?;
                    debug!("completed all authorizations");
                    Order::Ready { finalize }
                }
                Order::Ready { finalize } => {
                    debug!("sending csr");
                    let csr = cert.serialize_request_der()?;
                    account.finalize(finalize, csr).await?
                }
                Order::Valid { certificate } => {
                    debug!("download certificate");
                    let acme_cert_pem = account.certificate(certificate).await?;
                    let pems = pem::parse_many(&acme_cert_pem);
                    let cert_chain = pems
                        .into_iter()
                        .map(|p| RustlsCertificate(p.contents))
                        .collect();
                    let cert_key = CertifiedKey::new(cert_chain, Arc::new(pk));
                    self.cert_key.lock().unwrap().replace(cert_key.clone());
                    let pk_pem = cert.serialize_private_key_pem();
                    Self::save_certified_key(cache_dir, file_name, pk_pem, acme_cert_pem).await;
                    return Ok(());
                }
                Order::Invalid => return Err(OrderErrorKind::BadOrder(order).into()),
            }
        }
    }

    async fn authorize(&self, account: &Account, url: &String) -> Result<(), OrderError> {
        let (domain, challenge_url) = match account.auth(url).await? {
            Auth::Pending {
                identifier,
                challenges,
            } => {
                let Identifier::Dns(domain) = identifier;
                info!("trigger challenge for {}", &domain);
                let (challenge, auth_key) = account.tls_alpn_01(&challenges, domain.clone())?;

                self.dio.store(CertificateChallenge {
                    cert: CertificateKey {
                        domain: domain.clone(),
                        pk: auth_key.key.
                    }
                })?;
                self.dio.commit().await?;

                self.auth_keys
                    .lock()
                    .unwrap()
                    .insert(domain.clone(), auth_key);
                account.challenge(&challenge.url).await?;
                (domain, challenge.url.clone())
            }
            Auth::Valid => return Ok(()),
            auth => return Err(OrderErrorKind::BadAuth(auth).into()),
        };
        for i in 0u64..5 {
            tokio::time::sleep(Duration::from_secs(1 << i)).await;
            match account.auth(url).await? {
                Auth::Pending { .. } => {
                    info!("authorization for {} still pending", &domain);
                    account.challenge(&challenge_url).await?
                }
                Auth::Valid => return Ok(()),
                auth => return Err(OrderErrorKind::BadAuth(auth).into()),
            }
        }
        Err(OrderErrorKind::TooManyAttemptsAuth(domain).into())
    }
    */
}

impl ResolvesServerCert
for Acme
{
    fn resolve(&self, client_hello: ClientHello) -> Option<CertifiedKey> {
        if let Some(sni) = client_hello.server_name() {
            let sni = sni.to_owned();
            let sni: String = AsRef::<str>::as_ref(&sni).to_string();

            if client_hello.alpn() == Some(&[ACME_TLS_ALPN_NAME]) {
                let guard = self.auths.read();
                if let Some(cert) = guard.get(&sni)  {
                    trace!("tls_challenge: auth_hit={:?}", sni);
                    return Some(cert.clone());
                } else {
                    trace!("tls_challenge: auth_miss={:?}", sni);
                    return None;
                }
            }

            let guard = self.certs.read();
            let ret = if let Some(cert) = guard.get(&sni)  {
                trace!("tls_hello: cert_hit={:?}", sni);
                Some(cert.clone())
            } else {
                trace!("tls_hello: cert_miss={:?}", sni);
                None
            };

            {
                let tx = self.touch_tx.clone();
                TaskEngine::spawn(async move {
                    let _ = tx.send(sni).await;
                });
            }
            return ret;
        } else {
            debug!("rejected connection (SNI was missing)");
        }
        None
    }
}