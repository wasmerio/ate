#[allow(unused_imports)]
use log::{info, error, debug};

use crate::event::*;

use tokio::sync::mpsc;

pub(crate) struct ChainListener
{
    pub(crate) id: u64,
    pub(crate) sender: mpsc::Sender<EventData>
}