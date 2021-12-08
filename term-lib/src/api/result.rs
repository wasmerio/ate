use tokio::sync::mpsc;

pub struct AsyncResult<T> {
    pub(crate) rx: mpsc::Receiver<T>,
}

impl<T> AsyncResult<T> {
    pub fn new(rx: mpsc::Receiver<T>) -> Self {
        Self { rx }
    }

    pub fn block_on(mut self) -> Option<T> {
        self.rx.blocking_recv()
    }

    pub async fn join(mut self) -> Option<T> {
        self.rx.recv().await
    }
}