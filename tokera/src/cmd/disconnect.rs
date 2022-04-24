#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::bus::disconnect_from_networks;

pub async fn main_opts_disconnect()
{
    disconnect_from_networks().await;
}
