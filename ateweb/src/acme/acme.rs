#[allow(unused_imports, dead_code)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use error_chain::bail;
use tokio_rustls::rustls::sign::{any_ecdsa_type, CertifiedKey};
use tokio_rustls::rustls::PrivateKey;
use base64::URL_SAFE_NO_PAD;
use rcgen::{Certificate, CustomExtension, PKCS_ECDSA_P256_SHA256};
use ring::signature::{EcdsaKeyPair, ECDSA_P256_SHA256_FIXED_SIGNING};
use ring::rand::SystemRandom;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use http::HeaderMap;
use http::HeaderValue;

use hyper::Request;
use hyper::Method;
use hyper::Client;
use hyper::Body;
use hyper_tls::HttpsConnector;

use crate::error::*;
use super::security::*;

pub const LETS_ENCRYPT_STAGING_DIRECTORY: &str =
    "https://acme-staging-v02.api.letsencrypt.org/directory";
pub const LETS_ENCRYPT_PRODUCTION_DIRECTORY: &str =
    "https://acme-v02.api.letsencrypt.org/directory";
pub const PEBBLE_DIRECTORY: &str =
    "https://localhost:14000/dir";
pub const ACME_TLS_ALPN_NAME: &[u8] = b"acme-tls/1";

#[derive(Debug)]
pub struct Account {
    pub key_pair: EcdsaKeyPair,
    pub directory: Directory,
    pub kid: String,
}

impl Account
{
    pub async fn load_or_create<'a, S, I>(
        directory: Directory,
        contact: I,
    ) -> Result<Self, AcmeError>
    where S: AsRef<str> + 'a,
          I: IntoIterator<Item = &'a S>,
    {
        let alg = &ECDSA_P256_SHA256_FIXED_SIGNING;
        let contact: Vec<&'a str> = contact.into_iter().map(AsRef::<str>::as_ref).collect();
        
        let key_pair = {
            info!("creating a new account key");
            let rng = SystemRandom::new();
            let pkcs8 = EcdsaKeyPair::generate_pkcs8(alg, &rng)?;
            EcdsaKeyPair::from_pkcs8(alg, pkcs8.as_ref())?
        };

        let payload = json!({
            "termsOfServiceAgreed": true,
            "contact": contact,
        }).to_string();

        let body = sign(
            &key_pair,
            None,
            directory.nonce().await?,
            &directory.new_account,
            &payload,
        )?;
        
        let (_, headers) = api_call(&directory.new_account, Method::POST, Some(body), directory.insecure).await?;
        let kid = get_header(&headers, "Location")?;

        Ok(Account {
            key_pair,
            kid,
            directory,
        })
    }

    pub async fn request(&self, url: &str, payload: &str) -> Result<String, AcmeError> {
        let mut n = 0;
        loop {
            let body = sign(
                &self.key_pair,
                Some(&self.kid),
                self.directory.nonce().await?,
                url,
                payload,
            )?;

            match api_call(url, Method::POST, Some(body), self.directory.insecure).await {
                Ok((body, _)) => {
                    debug!("response: {:?}", body);
                    return Ok(body)
                },
                Err(AcmeError(AcmeErrorKind::ApiError(err), _)) => {
                    if err.typ == "urn:ietf:params:acme:error:badNonce" && n < 5 {
                        n += 1;
                        continue;
                    }
                    bail!(AcmeErrorKind::ApiError(err));
                },
                Err(err) => {
                    return Err(err);
                }
            };            
        }
    }

    pub async fn auth(&self, url: &str) -> Result<Auth, AcmeError> {
        let payload = "".to_string();
        let response = self.request(url, &payload).await;
        Ok(serde_json::from_str(&response?)?)
    }

    pub async fn challenge(&self, url: &str) -> Result<(), AcmeError> {
        self.request(url, "{}").await?;
        Ok(())
    }

    pub async fn new_order(&self, domains: Vec<String>) -> Result<Order, AcmeError> {
        let domains: Vec<Identifier> = domains.into_iter().map(|d| Identifier::Dns(d)).collect();
        let payload = format!("{{\"identifiers\":{}}}", serde_json::to_string(&domains)?);
        let response = self.request(&self.directory.new_order, &payload).await;
        Ok(serde_json::from_str(&response?)?)
    }

    pub async fn finalize(&self, url: &str, csr: Vec<u8>) -> Result<Order, AcmeError> {
        let payload = format!(
            "{{\"csr\":\"{}\"}}",
            base64::encode_config(csr, URL_SAFE_NO_PAD)
        );
        let response = self.request(url, &payload).await;
        Ok(serde_json::from_str(&response?)?)
    }

    pub async fn certificate(&self, url: &str) -> Result<String, AcmeError> {
        self.request(url, "").await
    }

    pub async fn check(&self, url: &str) -> Result<Order, AcmeError> {
        let response = self.request(url, "").await;
        Ok(serde_json::from_str(&response?)?)
    }

    pub fn tls_alpn_01<'a>(
        &self,
        challenges: &'a Vec<Challenge>,
        domain: String,
    ) -> Result<(&'a Challenge, CertifiedKey), AcmeError>
    {
        let challenge = challenges
            .iter()
            .filter(|c| c.typ == ChallengeType::TlsAlpn01)
            .next();

        let challenge = match challenge {
            Some(challenge) => challenge,
            None => return Err(AcmeErrorKind::NoTlsAlpn01Challenge.into()),
        };

        let mut params = rcgen::CertificateParams::new(vec![domain]);
        let key_auth = key_authorization_sha256(&self.key_pair, &*challenge.token)?;
        params.alg = &PKCS_ECDSA_P256_SHA256;
        params.custom_extensions = vec![CustomExtension::new_acme_identifier(key_auth.as_ref())];

        let cert = Certificate::from_params(params)?;
        let pk = any_ecdsa_type(&PrivateKey(cert.serialize_private_key_der())).unwrap();
        let certified_key = CertifiedKey::new(
            vec![tokio_rustls::rustls::Certificate(cert.serialize_der()?)],
            Arc::new(pk),
        );

        Ok((challenge, certified_key))
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Directory {
    pub new_nonce: String,
    pub new_account: String,
    pub new_order: String,
    #[serde(skip)]
    pub insecure: bool
}

impl Directory
{
    pub async fn discover(url: &str) -> Result<Self, AcmeError> {
        let insecure = url == PEBBLE_DIRECTORY;
        let (body, _) = api_call(url, Method::GET, None, insecure).await?;
        let mut ret: Directory = serde_json::from_str(body.as_str())?;
        ret.insecure = insecure;
        Ok(ret)
    }

    pub async fn nonce(&self) -> Result<String, AcmeError> {
        let (_, headers) = api_call(&self.new_nonce.as_str(), Method::HEAD, None, self.insecure).await?;
        get_header(&headers, "replay-nonce")
    }
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub enum ChallengeType {
    #[serde(rename = "http-01")]
    Http01,
    #[serde(rename = "dns-01")]
    Dns01,
    #[serde(rename = "tls-alpn-01")]
    TlsAlpn01,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum Order {
    Pending {
        authorizations: Vec<String>,
        finalize: String,
    },
    Ready {
        finalize: String,
    },
    Valid {
        certificate: String,
    },
    Invalid,
    Processing {
        finalize: String,
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum Auth {
    Pending {
        identifier: Identifier,
        challenges: Vec<Challenge>,
    },
    Valid,
    Invalid,
    Revoked,
    Expired,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum Identifier {
    Dns(String),
}

#[derive(Debug, Deserialize)]
pub struct Challenge {
    #[serde(rename = "type")]
    pub typ: ChallengeType,
    pub url: String,
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct ApiError {
    #[serde(rename = "type")]
    pub typ: String,
    pub detail: String,
    pub status: u16,
}

fn get_header(response: &HeaderMap<HeaderValue>, header: &'static str) -> Result<String, AcmeError> {
    match response.get(header) {
        Some(value) => Ok(value.to_str()?.to_string()),
        None => bail!(AcmeErrorKind::MissingHeader(header)),
    }
}

async fn api_call(
    req_url: &str,
    method: Method,
    req: Option<String>,
    insecure: bool,
) -> Result<(String, HeaderMap<HeaderValue>), AcmeError>
{
    // Build the request
    let req_url = req_url.to_string();
    if let Some(req_str) = req.as_ref() {
        debug!("Request: {:?}@{}", req_str, req_url);
    } else {
        debug!("Request: @{}", req_url);
    }

    // Create the HTTPS client
    let client = {
        let tls_connector = hyper_tls::native_tls::TlsConnector::builder()
            .danger_accept_invalid_certs(insecure)
            .build()
            .unwrap();
        let mut http_connector = hyper::client::HttpConnector::new();
        http_connector.enforce_http(false);
        let https_connector = HttpsConnector::from((http_connector, tls_connector.into()));
        Client::builder().build::<_, hyper::Body>(https_connector)
    };

    // Make the request object
    let builder = Request::builder()
        .method(method)
        .uri(req_url)
        .header("Content-Type", "application/jose+json");

    let req = if let Some(req_str) = req {
        builder.body(Body::from(req_str)).unwrap()
    } else {
        builder.body(Body::empty()).unwrap()
    };

    let mut res = client.request(req).await?;
    let status = res.status();

    debug!("Response: {}", status);
    //debug!("Headers: {:#?}\n", res.headers());
    
    let headers = res.headers().clone();
    let res = hyper::body::to_bytes(res.body_mut()).await?;
    let orig_res = String::from_utf8(res.into_iter().collect()).unwrap();

    // Pretty print
    let res = match jsonxf::pretty_print(orig_res.as_str()) {
        Ok(a) => a,
        Err(err) => {
            error!("{}", err);
            orig_res.clone()
        }
    };
    
    // If an error occured then fail
    if !status.is_success() {
        warn!("{}", res);

        if status.as_u16() == 400 {
            if let Some(err) = serde_json::from_str::<ApiError>(res.as_str()).ok() {
                bail!(AcmeErrorKind::ApiError(err));
            }
        }
        bail!(AcmeErrorKind::BadResponse(status.as_u16(), orig_res));
    }

    debug!("Body: {}", res);
    Ok((res, headers))
}