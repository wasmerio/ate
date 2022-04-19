use std::ops::Deref;
use std::io::Read;
use ate::prelude::*;
use chrono::NaiveDateTime;
use error_chain::bail;
use async_stream::stream;
use futures_util::pin_mut;
use futures_util::stream::StreamExt;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::error::*;
use crate::model::{HistoricActivity, activities, InstanceHello, InstanceCommand, InstanceExport, InstanceCall};
use crate::opt::*;
use crate::api::{TokApi, InstanceClient};
use crate::bus::peer_with_network;

use super::*;

pub async fn main_opts_connect(
    opts: OptsConnectFor,
    token_path: String,
    auth_url: url::Url,
    db_url: url::Url,
    net_url: url::Url,
    ignore_certificate: bool
) -> Result<(), InstanceError>
{
    // Determine the instance authority from the session URL
    let mut instance_authority = net_url.domain()
        .map(|a| a.to_string())
        .unwrap_or_else(|| "tokera.sh".to_string());
    if instance_authority == "localhost" {
        instance_authority = "tokera.sh".to_string();
    }

    // Get the specifics around the network we will be connecting too
    let mut context = PurposeContext::new(&opts, token_path.as_str(), &auth_url, Some(&db_url), true).await?;
    let (instance, _) = context.api.instance_action(name).await?;
    let mut instance = instance?;
    let chain = instance.chain.clone();
    let access_token = instance.subnet.network_token.clone();
    
    // Now actually connect to this peer
    peer_with_network(net_url, chain, access_token).await?;
    Ok(())
}
