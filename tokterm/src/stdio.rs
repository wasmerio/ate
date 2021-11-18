#![allow(unused_imports)]
#![allow(dead_code)]
use std::fmt;
use std::future::Future;
use tokio::io::{self};
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace};

use crate::fs::TokeraSocket;
use crate::fs::UnionFileSystem;

use super::common::*;
use super::fd::*;
use super::state::*;
use super::tty::*;

#[derive(Debug, Clone)]
pub struct Stdio {
    pub stdin: Fd,
    pub stdout: Fd,
    pub stderr: Fd,
    pub tty: Tty,
}

impl Stdio {
    pub fn println(&self, fmt: fmt::Arguments) -> impl Future<Output = io::Result<usize>> {
        let data = format!("{}", fmt);
        let mut stdout = self.stdout.clone();
        async move { stdout.write(data.as_bytes()).await }
    }

    pub fn eprintln(&self, fmt: fmt::Arguments) -> impl Future<Output = io::Result<usize>> {
        let data = format!("{}", fmt);
        let mut stderr = self.stderr.clone();
        async move { stderr.write(data.as_bytes()).await }
    }
}
