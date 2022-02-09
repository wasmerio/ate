use async_trait::async_trait;
use ate_files::prelude::*;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::Mutex as AsyncMutex;
use atessh::term_lib;
use ate::comms::*;
use term_lib::api::ConsoleAbi;
use term_lib::api::ConsoleRect;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use tokio::sync::mpsc;

pub struct SessionHandler
{
    pub tx: AsyncMutex<Upstream>,
    pub native_files: Arc<FileAccessor>,
    pub rect: Arc<Mutex<ConsoleRect>>,
    pub exit: mpsc::Sender<()>,
}

#[async_trait]
impl ConsoleAbi
for SessionHandler
{
    async fn stdout(&self, data: Vec<u8>) {
        let mut tx = self.tx.lock().await;
        if data.len() > 0 {                            
            if let Err(err) = tx.outbox.send(&data[..]).await {
                debug!("writing failed - will close the channel now - {}", err);
                self.exit().await;
            }
        }
    }

    async fn stderr(&self, data: Vec<u8>) {
        self.stdout(data).await;
    }

    async fn flush(&self) {
    }

    async fn log(&self, text: String) {
        trace!("{}", text);
    }

    async fn console_rect(&self) -> ConsoleRect {
        let rect = self.rect.lock().unwrap();
        rect.deref().clone()
    }

    async fn cls(&self) {
        let txt = format!("{}[2J", 27 as char);
        let data = txt.as_bytes().to_vec();
        self.stdout(data).await;
    }

    async fn exit(&self) {
        {
            let mut tx = self.tx.lock().await;
            let _ = tx.outbox.close().await;
        }
        let _ = self.exit.send(()).await;
    }
}
