use tokio::sync::mpsc;
#[allow(unused_imports)]
use tracing::{debug, error, info, warn};

use crate::event::*;
use crate::header::*;

use super::*;

pub(crate) struct ChainSniffer {
    pub(crate) id: u64,
    pub(crate) filter: Box<dyn Fn(&EventWeakData) -> bool + Send + Sync>,
    pub(crate) notify: mpsc::Sender<PrimaryKey>,
}

impl ChainSniffer {
    pub(super) fn convert(&self, key: PrimaryKey) -> Notify {
        Notify {
            key,
            who: NotifyWho::Sender(self.notify.clone()),
        }
    }
}
