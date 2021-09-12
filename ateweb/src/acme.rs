
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
    fn resolve(&self, _client_hello: ClientHello) -> Option<CertifiedKey> {
        None
    }
}