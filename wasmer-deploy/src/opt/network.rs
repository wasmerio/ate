use clap::Parser;
use url::Url;
use ate_comms::StreamSecurity;

use super::purpose::*;
use super::OptsCidrAction;
use super::OptsPeeringAction;

#[allow(dead_code)]
#[derive(Parser)]
#[clap(version = "1.5", author = "Wasmer Inc <info@wasmer.io>")]
pub struct OptsNetwork {
    #[clap(subcommand)]
    pub cmd: OptsNetworkCommand,
    /// URL where the data is remotely stored on a distributed commit log (e.g. wss://wasmer.sh/db).
    #[clap(short, long)]
    pub db_url: Option<Url>,
    /// Level of security to apply to the connection
    #[clap(long, default_value = "any")]
    pub security: StreamSecurity
}

#[derive(Parser)]
pub enum OptsNetworkCommand
{
    /// Performs an action on a particular grouping of networks or a specific network in a group
    #[clap()]
    For(OptsNetworkCommandFor),
    /// Reconnects to a network using an access token that was previously exported for a network
    #[clap()]
    Reconnect(OptsNetworkReconnect),
    /// Disconnects from the network
    #[clap()]
    Disconnect,
    /// Monitors the local network and sends the output to packet metadata to STDOUT
    #[clap()]
    Monitor(OptsNetworkMonitor),
    /// Create a TAP device that bridges the local network with the remote network
    #[cfg(feature = "enable_bridge")]
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[clap()]
    Bridge(OptsNetworkBridge),
}


#[allow(dead_code)]
#[derive(Parser, Clone)]
#[clap(version = "1.5", author = "Wasmer Inc <info@wasmer.io>")]
pub struct OptsNetworkCommandFor {
    /// Category of networks to perform an action upon
    #[clap(subcommand)]
    pub purpose: OptsNetworkPurpose,
}

#[allow(dead_code)]
#[derive(Parser, Clone)]
#[clap(version = "1.5", author = "Wasmer Inc <info@wasmer.io>")]
pub struct OptsNetworkMonitor {
    /// Overrides the URL where the network can be accessed from (e.g. wss://wasmer.sh/net)
    /// (the default is to use the URL contained within the token)
    #[clap(short, long)]
    pub net_url: Option<Url>,
}

#[cfg(feature = "enable_bridge")]
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[allow(dead_code)]
#[derive(Parser, Clone)]
#[clap(version = "1.5", author = "Wasmer Inc <info@wasmer.io>")]
pub struct OptsNetworkBridge {
    /// Overrides the URL where the network can be accessed from (e.g. wss://wasmer.sh/net)
    /// (the default is to use the URL contained within the token)
    #[clap(short, long)]
    pub net_url: Option<Url>,
    /// Port will receive all packets on the network and not just those destined for this port
    #[clap(short, long)]
    pub promiscuous: bool,
    /// Runs the port as a daemon in the background after forking the process
    #[clap(short, long)]
    pub daemon: bool,
    /// Sets the MTU for this device
    #[clap(long)]
    pub mtu: Option<u32>,
}

#[allow(dead_code)]
#[derive(Parser, Clone)]
#[clap(version = "1.5", author = "Wasmer Inc <info@wasmer.io>")]
pub struct OptsNetworkConnect {
    /// Name of the network to connect to
    #[clap(index = 1)]
    pub name: String,
    /// Exports the token to STDOUT rather than storing it so that it may be used later to reconnect
    #[clap(short, long)]
    pub export: bool,
}

#[allow(dead_code)]
#[derive(Parser, Clone)]
#[clap(version = "1.5", author = "Wasmer Inc <info@wasmer.io>")]
pub struct OptsNetworkReconnect {
    /// Reconnects to a network using an access token that was previously exported
    #[clap(index = 1)]
    pub token: String,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsNetworkCidr {
    /// Name of the network to be modify routing tables
    #[clap(index = 1)]
    pub name: String,
    /// Action to perform on the cidr
    #[clap(subcommand)]
    pub action: OptsCidrAction,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsNetworkPeering {
    /// Name of the network to modify peering
    #[clap(index = 1)]
    pub name: String,
    /// Action to perform on the peerings
    #[clap(subcommand)]
    pub action: OptsPeeringAction,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsNetworkCreate {
    /// Name of the new network (which will be generated if you dont supply one)
    #[clap(index = 1)]
    pub name: Option<String>,
    /// Forces the creation of this network even if there is a duplicate
    #[clap(short, long)]
    pub force: bool,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsNetworkKill {
    /// Name of the network to be killed
    /// (killed networks are perminently destroyed)
    #[clap(index = 1)]
    pub name: String,
    /// Forces the removal of the network from the wallet even
    /// if access is denied to its data and thus this would create an orphan chain.
    #[clap(short, long)]
    pub force: bool,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsNetworkReset {
    /// Name of the network to destroy
    #[clap(index = 1)]
    pub name: String,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsNetworkDetails {
    /// Name of the network to get the details for
    #[clap(index = 1)]
    pub name: String,
}

#[derive(Parser, Clone)]
#[clap()]
pub enum OptsNetworkAction {
    /// List all the networks that can be connected to
    #[clap()]
    List,
    /// Details the details of a particular network
    #[clap()]
    Details(OptsNetworkDetails),
    /// List, add or remove a CIDR (subnet)
    #[clap()]
    Cidr(OptsNetworkCidr),
    /// List, add or remove a network peering between different networks
    #[clap()]
    Peering(OptsNetworkPeering),
    /// Connects this machine/instance with a remote network
    #[clap()]
    Connect(OptsNetworkConnect),
    /// Creates a new network
    #[clap()]
    Create(OptsNetworkCreate),
    /// Kills are particular network
    #[clap()]
    Kill(OptsNetworkKill),
    /// Resets an network and its attached mesh nodes
    #[clap()]
    Reset(OptsNetworkReset),
}

#[derive(Parser, Clone)]
pub enum OptsNetworkPurpose {
    /// Networks associated to you personally
    #[clap()]
    Personal(OptsNetworkForPersonal),
    /// Networks associated with a particular group you can authorize on behalf of
    #[clap()]
    Domain(OptsNetworkForDomain),
}

impl OptsNetworkPurpose {
    pub fn is_personal(&self) -> bool {
        if let OptsNetworkPurpose::Personal(..) = self {
            true
        } else {
            false
        }
    }
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsNetworkForPersonal {
    /// Name of the personal wallet to use for this network (if required)
    #[clap(index = 1, default_value = "default")]
    pub wallet_name: String,
    /// Action to perform on this network
    #[clap(subcommand)]
    pub action: OptsNetworkAction,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsNetworkForDomain {
    /// Name of the group that the network is attached to
    #[clap(index = 1)]
    pub domain: String,
    /// Name of the group wallet to use in this context (if required)
    #[clap(index = 2, default_value = "default")]
    pub wallet_name: String,
    /// Action to perform on this network
    #[clap(subcommand)]
    pub action: OptsNetworkAction,
}

impl OptsPurpose<OptsNetworkAction> for OptsNetworkPurpose {
    fn purpose(&self) -> Purpose<OptsNetworkAction> {
        match self {
            OptsNetworkPurpose::Personal(a) => Purpose::Personal {
                wallet_name: a.wallet_name.clone(),
                action: a.action.clone(),
            },
            OptsNetworkPurpose::Domain(a) => Purpose::Domain {
                domain_name: a.domain.clone(),
                wallet_name: a.wallet_name.clone(),
                action: a.action.clone(),
            },
        }
    }
}
