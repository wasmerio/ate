#[allow(unused_imports)]
use tracing::{info, error, warn, debug};
use tokio::sync::mpsc;

use crate::{event::*};
use crate::header::*;

use super::*;

pub(crate) struct ChainSniffer
{
    pub(crate) id: u64,
    pub(crate) filter: Box<dyn Fn(&EventData) -> bool + Send + Sync>,
    pub(crate) notify: mpsc::Sender<PrimaryKey>,
}

impl ChainSniffer
{
    pub(super) fn convert(&self, key: PrimaryKey) -> Notify {
        Notify {
            key,
            who: NotifyWho::Sender(self.notify.clone())
        }
    }
}