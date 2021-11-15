use std::sync::Arc;
use tokio::sync::mpsc;
#[allow(unused_imports)]
use tracing::{debug, error, info, warn};

use crate::error::*;
use crate::header::*;

use super::*;

pub(crate) enum NotifyWho {
    Sender(mpsc::Sender<PrimaryKey>),
    Service(Arc<dyn Service>),
}

impl std::fmt::Debug for NotifyWho {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "notify-who:sender")
    }
}

#[derive(Debug)]
pub(crate) struct Notify {
    pub(crate) key: PrimaryKey,
    pub(crate) who: NotifyWho,
}

impl Notify {
    pub(crate) async fn notify(self) -> Result<(), InvokeError> {
        match self.who {
            NotifyWho::Sender(sender) => sender.send(self.key).await?,
            NotifyWho::Service(service) => service.notify(self.key).await?,
        }
        Ok(())
    }
}
