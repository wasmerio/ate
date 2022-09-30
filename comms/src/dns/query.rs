use std::net::IpAddr;
use std::str::FromStr;
use ate_crypto::AteHash;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

pub use trust_dns_client::rr::*;

impl super::Dns
{
    pub async fn dns_certs(&mut self, name: &str) -> Vec<AteHash> {
        match name.to_lowercase().as_str() {
            "localhost" => {
                return Vec::new();
            }
            _ => {}
        };

        if let Ok(_) = IpAddr::from_str(name) {
            return Vec::new();
        }

        trace!("dns_query for {}", name);
        
        let mut txts = Vec::new();
        if let Some(response) = self
            .query(Name::from_str(name).unwrap(), DNSClass::IN, RecordType::TXT)
            .await
            .ok()
        {
            for answer in response.answers() {
                if let RData::TXT(ref txt) = *answer.rdata() {
                    txts.push(txt.to_string());
                }
            }
        }

        let prefix = "ate-cert-";

        let mut certs = Vec::new();
        for txt in txts {
            let txt = txt.replace(" ", "");
            if txt.trim().starts_with(prefix) {
                let start = prefix.len();
                let hash = &txt.trim()[start..];
                if let Some(hash) = AteHash::from_hex_string(hash) {
                    trace!("found certificate({}) for {}", hash, name);
                    certs.push(hash);
                }
            }
        }
        trace!(
            "dns_query for {} returned {} certificates",
            name,
            certs.len()
        );

        certs
    }
}