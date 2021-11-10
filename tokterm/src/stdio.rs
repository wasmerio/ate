#![allow(unused_imports)]
#![allow(dead_code)]
#[allow(unused_imports, dead_code)]
use tracing::{info, error, debug, trace};
use std::fmt;
use std::future::Future;
use tokio::io::{self};

use crate::fs::TokeraSocketFactory;
use crate::fs::UnionFileSystem;

use super::common::*;
use super::fd::*;
use super::state::*;
use super::tty::*;

#[derive(Debug, Clone)]
pub struct Stdio
{
    pub stdin: Fd,
    pub stdout: Fd,
    pub stderr: Fd,
    pub tty: Tty,
    pub tok: TokeraSocketFactory,
    pub root: UnionFileSystem,
}

impl Stdio
{
    pub fn println(&self, fmt: fmt::Arguments) -> impl Future<Output=io::Result<usize>> {
        let data = format!("{}", fmt);
        let mut stdout = self.stdout.clone();
        async move {
            stdout.write(data.as_bytes()).await
        }
    }

    pub fn eprintln(&self, fmt: fmt::Arguments) -> impl Future<Output=io::Result<usize>> {
        let data = format!("{}", fmt);
        let mut stderr = self.stderr.clone();
        async move {
            stderr.write(data.as_bytes()).await
        }
    }
}