use std::sync::RwLock;
use once_cell::sync::Lazy;
use ate_crypto::AteHash;

pub static GLOBAL_CERTIFICATES: Lazy<RwLock<Vec<AteHash>>> =
    Lazy::new(|| RwLock::new(Vec::new()));

pub fn add_global_certificate(cert: &AteHash) {
    GLOBAL_CERTIFICATES.write().unwrap().push(cert.clone());
}

pub fn get_global_certificates() -> Vec<AteHash> {
    let mut ret = GLOBAL_CERTIFICATES.read().unwrap().clone();
    ret.push(AteHash::from_hex_string("f0a961c31f83c758ff0b669cc61b0f76").unwrap());
    ret
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CertificateValidation {
    DenyAll,
    AllowAll,
    AllowedCertificates(Vec<AteHash>),
}

impl CertificateValidation {
    pub fn validate(&self, cert: &AteHash) -> bool {
        match self {
            CertificateValidation::DenyAll => false,
            CertificateValidation::AllowAll => true,
            CertificateValidation::AllowedCertificates(a) => a.contains(cert),
        }
    }
}
