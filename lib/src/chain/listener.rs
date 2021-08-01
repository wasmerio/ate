#[allow(unused_imports)]
use tracing::{info, debug, warn, error, trace};

use crate::event::*;

use tokio::sync::mpsc;

#[derive(Debug)]
pub(crate) struct ChainListener
{
    pub(crate) id: u64,
    pub(crate) sender: mpsc::Sender<EventData>
}