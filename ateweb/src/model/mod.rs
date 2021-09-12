mod web_conf;
mod cert;

pub use web_conf::*;
pub use cert::*;

pub const CERT_STORE_ID: u64 = 7127953076879823547u64;
pub const CERT_STORE_GROUP_NAME: &'static str = "cert.tokera.com";