#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};

use crate::event::*;

use tokio::sync::mpsc;

#[derive(Debug)]
pub(crate) struct ChainListener
{
    pub(crate) id: u64,
    pub(crate) sender: mpsc::Sender<EventData>
}