use async_trait::async_trait;
use std::ops::Deref;
use std::ops::DerefMut;
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

use tokera::model::InstanceReply;

pub enum SessionTx {
    None,
    Upstream(Upstream),
    Feeder(mpsc::Sender<InstanceReply>),
}

pub struct SessionHandler
{
    pub tx: AsyncMutex<SessionTx>,
    pub rect: Arc<Mutex<ConsoleRect>>,
    pub exit: mpsc::Sender<()>,
}

impl SessionHandler
{
    async fn stdout_internal(&self, data: Vec<u8>, is_err: bool) {
        let mut tx = self.tx.lock().await;
        if data.len() > 0 {
            match tx.deref_mut() {
                SessionTx::None => { }
                SessionTx::Upstream(tx) => {
                    if let Err(err) = tx.outbox.write(&data[..]).await {
                        debug!("writing failed - will close the channel now - {}", err);
                        self.exit().await;
                    }
                }
                SessionTx::Feeder(tx) => {
                    let msg = if is_err {
                        InstanceReply::Stderr { data }
                    } else {
                        InstanceReply::Stdout { data }
                    };
                    let _ = tx.send(msg).await;
                }
            }
        }
    }
}

#[async_trait]
impl ConsoleAbi
for SessionHandler
{
    async fn stdout(&self, data: Vec<u8>) {
        self.stdout_internal(data, false).await;
    }

    async fn stderr(&self, data: Vec<u8>) {
        self.stdout_internal(data, true).await;
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
            match tx.deref_mut() {
                SessionTx::None => { }
                SessionTx::Upstream(tx) => {
                    let _ = tx.close().await;
                }
                SessionTx::Feeder(tx) => {
                    let _ = tx.send(InstanceReply::Exit).await;
                }
            }
        }
        let _ = self.exit.send(()).await;
    }
}
