#[allow(unused_imports)]
use log::{info, error, warn, debug};
use tokio::sync::mpsc;
use std::sync::Arc;

use crate::{error::*};
use crate::header::*;

use super::*;

pub(crate) enum NotifyWho
{
    Sender(mpsc::Sender<PrimaryKey>),
    Service(Arc<dyn Service>)
}

pub(crate) struct Notify
{
    pub(crate) key: PrimaryKey,
    pub(crate) who: NotifyWho,
}

impl Notify
{
    pub(crate) async fn notify(self) -> Result<(), ServiceError<()>> {
        match self.who {
            NotifyWho::Sender(sender) => sender.send(self.key).await?,
            NotifyWho::Service(service) => service.notify(self.key).await?
        }
        Ok(())
    }
}