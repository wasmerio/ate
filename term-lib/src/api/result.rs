use tokio::sync::mpsc;
use wasm_bus::abi::SerializationFormat;

pub struct AsyncResult<T> {
    pub(crate) rx: mpsc::Receiver<T>,
    pub(crate) format: SerializationFormat,
}

impl<T> AsyncResult<T> {
    pub fn new(format: SerializationFormat, rx: mpsc::Receiver<T>) -> Self {
        Self { rx, format }
    }

    pub fn block_on(mut self) -> Option<T> {
        self.rx.blocking_recv()
    }

    pub async fn join(mut self) -> Option<T> {
        self.rx.recv().await
    }
}
