#![allow(unused_imports)]
#![allow(dead_code)]
#[allow(unused_imports, dead_code)]
use tracing::{info, error, debug, trace, warn};
use std::ops::{Deref, DerefMut};
use std::io::Write;

use super::fd::*;
use super::tty::*;
use super::cconst::*;
use super::common::*;

#[derive(Debug, Clone)]
pub struct Stdout
{
    fd: Fd
}

impl Deref
for Stdout
{
    type Target = Fd;

    fn deref(&self) -> &Self::Target {
        &self.fd
    }
}

impl DerefMut
for Stdout
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.fd
    }
}

impl Stdout
{
    pub fn new(fd: Fd) -> Stdout {
        Stdout {
            fd
        }
    }

    pub async fn draw(&mut self, data: &str) {
        if let Err(err) = self.fd.write(data.as_bytes()).await {
            warn!("stdout-err: {}", err);
        }
    }
}