#[allow(unused_imports, dead_code)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use rustls::ResolvesServerCert;
use rustls::ClientHello;
use rustls::sign::CertifiedKey;

pub struct Acme
{
}

impl Acme
{
    pub fn new() -> Acme
    {
        Acme {
        }
    }
}

impl ResolvesServerCert
for Acme
{
    fn resolve(&self, client_hello: ClientHello) -> Option<CertifiedKey> {
        if let Some(from) = client_hello.server_name() {
            trace!("tls_hello: from={:?}", from);
        } else {
            debug!("rejected connection (SNI was missing)");
        }
        None
    }
}