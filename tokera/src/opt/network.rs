use clap::Parser;
use url::Url;

use super::purpose::*;

#[allow(dead_code)]
#[derive(Parser)]
#[clap(version = "1.5", author = "Tokera Pty Ltd <info@tokera.com>")]
pub struct OptsNetwork {
    #[clap(subcommand)]
    pub action: NetworkAction,
}

#[derive(Parser)]
pub enum NetworkAction
{
    /// List all the networks that can be connected to
    #[clap()]
    List(OptsNetworkList),
    /// Peers this machine with a remote network
    #[clap()]
    Connect(OptsNetworkConnect),
    /// Reconnects to a network using an access token that was previously exported
    #[clap()]
    Reconnect(OptsNetworkReconnect),
    /// Disconnects from the network
    #[clap()]
    Disconnect,
    /// Create a TAP device that bridges the local network with the remote network
    #[cfg(feature = "enable_bridge")]
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[clap()]
    Bridge(OptsNetworkBridge),
}

#[cfg(feature = "enable_bridge")]
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[allow(dead_code)]
#[derive(Parser, Clone)]
#[clap(version = "1.5", author = "Tokera Pty Ltd <info@tokera.com>")]
pub struct OptsNetworkBridge {
    /// Port will receive all packets on the network and not just those destined for this port
    #[clap(short, long)]
    pub promiscuous: bool,
    /// Runs the port as a daemon in the background after forking the process
    #[clap(short, long)]
    pub daemon: bool,
    /// URL where the network can be accessed from (e.g. wss://tokera.sh/net)
    #[clap(short, long)]
    pub net_url: Option<Url>,
    /// Sets a static IP address for this device rather than using DHCP
    #[clap(long)]
    pub ip4: Option<std::net::Ipv4Addr>,
    /// Sets a netmask for this device rather than using DHCP to determine it
    #[clap(long)]
    pub netmask4: Option<std::net::Ipv4Addr>,
    /// Sets the MTU for this device
    #[clap(long)]
    pub mtu: Option<u32>,
}

#[allow(dead_code)]
#[derive(Parser, Clone)]
#[clap(version = "1.5", author = "Tokera Pty Ltd <info@tokera.com>")]
pub struct OptsNetworkConnect {
    /// Category of network to connect to
    #[clap(subcommand)]
    pub purpose: OptsNetworkConnectFor,
    /// URL where the data is remotely stored on a distributed commit log (e.g. wss://tokera.sh/db).
    #[clap(short, long)]
    pub db_url: Option<Url>,
    /// URL where the network can be accessed from (e.g. ws://tokera.sh/net)
    #[clap(short, long)]
    pub net_url: Option<Url>,
    /// Indicates that the server certificate should be ignored
    #[clap(long)]
    pub ignore_certificate: bool,
    /// Exports the token to STDOUT rather than stored it so that it may be used later to reconnect
    #[clap(short, long)]
    pub export: bool,
    /// Encrypts the connection with both classical and quantum resistant encryption
    #[clap(long)]
    pub double_encrypt: bool,
}

#[allow(dead_code)]
#[derive(Parser, Clone)]
#[clap(version = "1.5", author = "Tokera Pty Ltd <info@tokera.com>")]
pub struct OptsNetworkReconnect {
    /// Reconnects to a network using an access token that was previously exported
    #[clap(index = 1)]
    pub token: String,
}

#[allow(dead_code)]
#[derive(Parser, Clone)]
#[clap(version = "1.5", author = "Tokera Pty Ltd <info@tokera.com>")]
pub struct OptsNetworkList {
    /// Category of network to list
    #[clap(subcommand)]
    pub purpose: OptsNetworkListFor,
    /// URL where the data is remotely stored on a distributed commit log (e.g. wss://tokera.sh/db).
    #[clap(short, long)]
    pub db_url: Option<Url>,
}

#[derive(Parser, Clone)]
pub enum OptsNetworkConnectFor {
    /// Networks associated to you personally
    #[clap()]
    Personal(OptsNetworkConnectForPersonal),
    /// Networks associated with a particular group you can authorize on behalf of
    #[clap()]
    Domain(OptsNetworkConnectForDomain),
}

impl OptsNetworkConnectFor {
    pub fn is_personal(&self) -> bool {
        if let OptsNetworkConnectFor::Personal(..) = self {
            true
        } else {
            false
        }
    }

    pub fn network_name(&self) -> &str {
        match self {
            OptsNetworkConnectFor::Personal(opts) => opts.network_name.as_str(),
            OptsNetworkConnectFor::Domain(opts) => opts.network_name.as_str()
        }
    }
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsNetworkConnectForPersonal {
    /// Name of the network (a.k.a. instance) to connect to
    #[clap(index = 1)]
    pub network_name: String,
    /// Name of the personal wallet to use for this network (if required)
    #[clap(index = 2, default_value = "default")]
    pub wallet_name: String,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsNetworkConnectForDomain {
    /// Name of the group that the network is attached to
    #[clap(index = 1)]
    pub domain: String,
    /// Name of the network (a.k.a. instance) to connect to
    #[clap(index = 2)]
    pub network_name: String,
    /// Name of the group wallet to use in this context (if required)
    #[clap(index = 3, default_value = "default")]
    pub wallet_name: String,
}

impl OptsPurpose<()> for OptsNetworkConnectFor {
    fn purpose(&self) -> Purpose<()> {
        match self {
            OptsNetworkConnectFor::Personal(a) => Purpose::Personal {
                wallet_name: a.wallet_name.clone(),
                action: (),
            },
            OptsNetworkConnectFor::Domain(a) => Purpose::Domain {
                domain_name: a.domain.clone(),
                wallet_name: a.wallet_name.clone(),
                action: (),
            },
        }
    }
}

#[derive(Parser, Clone)]
pub enum OptsNetworkListFor {
    /// Networks associated to you personally
    #[clap()]
    Personal(OptsNetworkListForPersonal),
    /// Networks associated with a particular group you can authorize on behalf of
    #[clap()]
    Domain(OptsNetworkListForDomain),
}

impl OptsNetworkListFor {
    pub fn is_personal(&self) -> bool {
        if let OptsNetworkListFor::Personal(..) = self {
            true
        } else {
            false
        }
    }
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsNetworkListForPersonal {
    /// Name of the personal wallet to use for this network (if required)
    #[clap(index = 1, default_value = "default")]
    pub wallet_name: String,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsNetworkListForDomain {
    /// Name of the group that the network is attached to
    #[clap(index = 1)]
    pub domain: String,
    /// Name of the group wallet to use in this context (if required)
    #[clap(index = 2, default_value = "default")]
    pub wallet_name: String,
}

impl OptsPurpose<()> for OptsNetworkListFor {
    fn purpose(&self) -> Purpose<()> {
        match self {
            OptsNetworkListFor::Personal(a) => Purpose::Personal {
                wallet_name: a.wallet_name.clone(),
                action: (),
            },
            OptsNetworkListFor::Domain(a) => Purpose::Domain {
                domain_name: a.domain.clone(),
                wallet_name: a.wallet_name.clone(),
                action: (),
            },
        }
    }
}
