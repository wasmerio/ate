use serde::*;
use std::sync::Arc;
use wasmer_bus::macros::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtyRect
{
    pub cols: u32,
    pub rows: u32,
}

#[wasmer_bus(format = "bincode")]
pub trait Tty {
    async fn stdin(
        &self,
        recv: impl Fn(Vec<u8>),
        flush: impl Fn(()),
    ) -> Arc<dyn Stdin>;

    async fn stdout(
        &self,
    ) -> Arc<dyn Stdout>;

    async fn stderr(
        &self,
    ) -> Arc<dyn Stderr>;

    async fn rect(
        &self,
    ) -> TtyRect;
}

#[wasmer_bus(format = "bincode")]
pub trait Stdin {
}

#[wasmer_bus(format = "bincode")]
pub trait Stdout {
    async fn write(&self, data: Vec<u8>) -> WriteResult;
    async fn flush(&self);
}

#[wasmer_bus(format = "bincode")]
pub trait Stderr {
    async fn write(&self, data: Vec<u8>) -> WriteResult;
    async fn flush(&self);
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WriteResult {
    Success(usize),
    Failed(String),
}