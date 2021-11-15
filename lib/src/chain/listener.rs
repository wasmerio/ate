#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use crate::event::*;

use tokio::sync::mpsc;

#[derive(Debug)]
pub(crate) struct ChainListener {
    pub(crate) id: u64,
    pub(crate) sender: mpsc::Sender<EventData>,
}
