#[allow(unused_imports)]
use tracing::{debug, error, info, warn};

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::loader::*;

pub struct RedoLogLoader {
    feed: mpsc::Sender<LoadData>,
}

impl RedoLogLoader {
    pub fn new() -> (Box<RedoLogLoader>, mpsc::Receiver<LoadData>) {
        let (tx, rx) = mpsc::channel(1000);
        let loader = RedoLogLoader { feed: tx };
        (Box::new(loader), rx)
    }
}

#[async_trait]
impl Loader for RedoLogLoader {
    async fn feed_load_data(&mut self, data: LoadData) {
        let _ = self.feed.send(data).await;
    }
}
