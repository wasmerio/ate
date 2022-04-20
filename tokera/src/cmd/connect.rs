#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::error::*;
use crate::opt::*;
use crate::bus::peer_with_network;

use super::*;

pub async fn main_opts_connect(
    opts: OptsConnectFor,
    token_path: String,
    auth_url: url::Url,
    db_url: url::Url,
    net_url: url::Url,
) -> Result<(), InstanceError>
{
    // Get the specifics around the network we will be connecting too
    let network_name = opts.network_name().to_string();
    let mut context = PurposeContext::new(&opts, token_path.as_str(), &auth_url, Some(&db_url), true).await?;
    let (instance, _) = context.api.instance_action(network_name.as_str()).await?;
    let instance = instance?;
    let chain = instance.chain.clone();
    let access_token = instance.subnet.network_token.clone();
    
    // Now actually connect to this peer
    peer_with_network(net_url, chain, access_token).await?;
    Ok(())
}
