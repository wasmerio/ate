#![allow(dead_code)]
use super::*;

#[derive(Debug, Default)]
pub struct ClientBuilder {
    pub(super) gzip: bool,
    pub(super) cors_proxy: Option<String>,
}

impl ClientBuilder {
    pub fn new() -> Self {
        ClientBuilder::default()
    }

    pub fn gzip(mut self, enable: bool) -> Self {
        self.gzip = enable;
        self
    }

    pub fn cors_proxy(mut self, domain: &str) -> Self {
        self.cors_proxy = Some(domain.to_string());
        self
    }

    pub fn build(self) -> Result<Client, Error> {
        Ok(Client { builder: self })
    }
}
