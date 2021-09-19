#[allow(unused_imports, dead_code)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use rustls::Certificate as RustlsCertificate;
use rustls::ResolvesServerCert;
use rustls::ClientHello;
use rustls::PrivateKey;
use rustls::sign::any_supported_type;
use rustls::sign::CertifiedKey;
use std::sync::Arc;
use ate::prelude::*;
use parking_lot::RwLock as StdRwLock;
use ttl_cache::TtlCache;
use bytes::Bytes;
use std::time::Duration;
use tokio::sync::Mutex;
use parking_lot::Mutex as StdMutex;
use fxhash::FxHashMap;
use std::collections::hash_map::Entry;
use x509_parser::parse_x509_certificate;
use rcgen::{CertificateParams, DistinguishedName, PKCS_ECDSA_P256_SHA256};
use super::acme::{
    Account,
    Auth,
    Directory,
    Identifier,
    Order,
    ACME_TLS_ALPN_NAME,
    LETS_ENCRYPT_PRODUCTION_DIRECTORY,
    LETS_ENCRYPT_STAGING_DIRECTORY,
};
use futures::future::try_join_all;

use crate::error::*;
use crate::repo::*;
use crate::model::*;

#[derive(Default)]
pub struct AcmeState
{
    err_cnt: i64,
    next_try: Option<chrono::DateTime<chrono::Utc>>,
}

pub struct AcmeResolver
{
    pub repo: Arc<Repository>,
    pub certs: StdRwLock<TtlCache<String, CertifiedKey>>,
    pub auths: StdRwLock<TtlCache<String, CertifiedKey>>,
    pub locks: StdMutex<FxHashMap<String, Arc<Mutex<AcmeState>>>>,
}

impl AcmeResolver
{
    pub async fn new(repo: &Arc<Repository>) -> Result<Arc<AcmeResolver>, AteError>
    {
        let ret = AcmeResolver {
            repo: Arc::clone(repo),
            certs: StdRwLock::new(TtlCache::new(65536usize)),
            auths: StdRwLock::new(TtlCache::new(1024usize)),
            locks: StdMutex::new(FxHashMap::default()),
        };
        Ok(Arc::new(ret))
    }
}

impl AcmeResolver
{
    async fn process_cert(&self, sni: &str, cert: Bytes, key: Bytes) -> Result<(), Box<dyn std::error::Error>>
    {
        let key = pem::parse(&key[..])?;
        let pems = pem::parse_many(&cert[..]);
        if pems.len() < 1 {
            error!("expected 1 or more pem in {}, got: {}", sni, pems.len());
            return Ok(());
        }
        let pk = match any_supported_type(&PrivateKey(key.contents)) {
            Ok(pk) => pk,
            Err(_) => {
                error!("{} does not contain an ecdsa private key", sni);
                return Ok(());
            }
        };
        let cert_chain: Vec<RustlsCertificate> = pems
            .into_iter()
            .map(|p| RustlsCertificate(p.contents))
            .collect();

        let cert_key = CertifiedKey::new(cert_chain, Arc::new(pk));

        let mut guard = self.certs.write();
        guard.insert(sni.to_string(), cert_key, Duration::from_secs(3600));

        Ok(())
    }

    pub async fn touch(&self, sni: String) -> Result<(), Box<dyn std::error::Error>>
    {
        // Fast path
        {
            let guard = self.certs.read();
            if let Some(cert) = guard.get(&sni) {
                let d = self.duration_until_renewal_attempt(cert);
                if d.as_secs() > 0 {
                    trace!("next renewal attempt in {}s", d.as_secs());
                    return Ok(())
                }
            }
        }

        let lock = {
            let mut guard = self.locks.lock();
            match guard.entry(sni.clone()) {
                Entry::Occupied(a) => {
                    Arc::clone(a.get())
                },
                Entry::Vacant(a) => {
                    let ret = Arc::new(Mutex::new(AcmeState::default()));
                    a.insert(Arc::clone(&ret));
                    ret
                }
            }
        };
        let mut lock = lock.lock().await;

        // Slow path
        let loaded = {
            let guard = self.certs.read();
            if let Some(cert) = guard.get(&sni) {
                let d = self.duration_until_renewal_attempt(cert);
                if d.as_secs() > 0 {
                    trace!("next renewal attempt in {}s", d.as_secs());
                    return Ok(())
                }
                true
            } else {
                false
            }
        };

        // If we have never loaded the certificates from disk then load them now
        if loaded == false {
            let cert = self.repo.get_file(sni.as_str(), WEB_CONF_FILES_CERT).await?;
            let key = self.repo.get_file(sni.as_str(), WEB_CONF_FILES_KEY).await?;
            if let Some(cert) = cert {
                if let Some(key) = key {
                    self.process_cert(sni.as_str(), cert, key).await?;
                } else {
                    warn!("missing certificate private key for {}", sni);
                }
            } else {
                warn!("missing certificate chain for {}", sni);
            }
        }

        // Check for exponental backoff
        if let Some(next_try) = lock.next_try {
            if next_try.gt(&chrono::Utc::now()) {
                trace!("aborting attempt due to exponential backoff");
                return Ok(())
            }
        }

        let production = false;
        let directory_url = match production {
            true => LETS_ENCRYPT_PRODUCTION_DIRECTORY,
            false => LETS_ENCRYPT_STAGING_DIRECTORY,
        };

        // Order the certificate using lets encrypt
        debug!("ordering of certificate started");
        match self
            .order(&directory_url, sni.as_str())
            .await
        {
            Ok(cert_key) => {
                debug!("successfully ordered certificate");
                lock.err_cnt = 0i64;
                lock.next_try = None;

                let mut guard = self.certs.write();
                guard.insert(sni.to_string(), cert_key, Duration::from_secs(3600));
            }
            Err(err) => {
                warn!("ordering certificate failed: {}", err);
                lock.err_cnt += 1i64;
                let retry_time = chrono::Duration::seconds(1 << lock.err_cnt);
                let retry_time = chrono::Utc::now() + retry_time;
                lock.next_try = Some(retry_time);
            }
        };

        Ok(())
    }

    fn duration_until_renewal_attempt(&self, cert_key: &CertifiedKey) -> Duration {
        let valid_until = match cert_key.cert.first() {
            Some(cert) => match parse_x509_certificate(cert.0.as_slice()) {
                Ok((_, cert)) => cert.validity().not_after.timestamp(),
                Err(err) => {
                    warn!("could not parse certificate: {}", err);
                    i64::MAX
                }
            },
            None => i64::MAX,
        };
        let valid_secs = (valid_until - chrono::Utc::now().timestamp()).max(0);
        Duration::from_secs(valid_secs as u64 / 2)
    }

    async fn order(
        &self,
        directory_url: &str,
        domain: &str,
    ) -> Result<CertifiedKey, OrderError>
    {
        let contacts = vec![ format!("mailto:info@{}", domain) ];
        let domains = vec![ domain.to_string() ];

        let mut params = CertificateParams::new(domains.clone());
        params.distinguished_name = DistinguishedName::new();
        params.alg = &PKCS_ECDSA_P256_SHA256;

        let cert = rcgen::Certificate::from_params(params)?;
        let pk = any_supported_type(&PrivateKey(cert.serialize_private_key_der())).unwrap();

        debug!("load_or_create account");
        let directory = Directory::discover(directory_url).await?;
        let account = Account::load_or_create(directory, &contacts).await?;

        debug!("new order for {:?}", domains);
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
                    account.finalize(finalize.as_str(), csr).await?
                }
                Order::Valid { certificate } => {
                    debug!("download certificate");
                    let acme_cert_pem = account.certificate(certificate.as_str()).await?;
                    let pems = pem::parse_many(&acme_cert_pem);
                    let cert_chain = pems
                        .into_iter()
                        .map(|p| RustlsCertificate(p.contents))
                        .collect();
                    let cert_key = CertifiedKey::new(cert_chain, Arc::new(pk));
                    return Ok(cert_key);
                }
                Order::Invalid => return Err(OrderErrorKind::BadOrder(order).into()),
            }
        }
    }

    async fn authorize(&self, account: &Account, url: &String) -> Result<(), OrderError> {
        debug!("starting authorization for {}", url);
        let (domain, challenge_url) = match account.auth(url).await? {
            Auth::Pending {
                identifier,
                challenges,
            } => {
                let Identifier::Dns(domain) = identifier;
                info!("trigger challenge for {}", &domain);
                let (challenge, auth_key) = account.tls_alpn_01(&challenges, domain.clone())?;

                self.auths
                    .write()
                    .insert(domain.clone(), auth_key, Duration::from_secs(300));
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
}

impl ResolvesServerCert
for AcmeResolver
{
    fn resolve(&self, client_hello: ClientHello) -> Option<CertifiedKey>
    {
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
            
            return if let Some(cert) = guard.get(&sni)  {
                trace!("tls_hello: cert_hit={:?}", sni);
                Some(cert.clone())
            } else {
                trace!("tls_hello: cert_miss={:?}", sni);
                None
            };
        } else {
            debug!("rejected connection (SNI was missing)");
        }
        None
    }
}