#![allow(dead_code)]
use super::*;

#[derive(Debug, Default)]
pub struct ClientBuilder
{
    gzip: bool
}

impl ClientBuilder
{
    pub fn new() -> Self {
        ClientBuilder::default()
    }

    pub fn gzip(mut self, enable: bool) -> Self {
        self.gzip = enable;
        self
    }

    pub fn build(self) -> Result<Client, Error> {
        Ok(
            Client {
                builder: self
            }
        )
    }
}