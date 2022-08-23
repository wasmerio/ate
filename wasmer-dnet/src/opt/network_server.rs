#[allow(unused_imports, dead_code)]
use tracing::{info, error, debug};
use ate::{prelude::*};
use clap::Parser;

/// Runs the networkserver
#[derive(Parser)]
pub struct OptsNetworkServer {
    /// IP address that the network server will listen on
    #[clap(short, long, default_value = "::")]
    pub listen: IpAddr,
    /// Port that the server will listen on for HTTP requests which are then turned into websocket
    #[clap(long)]
    pub http_port: Option<u16>,
    /// Forces Wasmer to listen on a specific port for HTTPS requests with generated certificates
    #[clap(long)]
    pub tls_port: Option<u16>,
    /// Port that the switches will listen on for peer-to-peer traffic (default: 2000)
    #[clap(long)]
    pub udp_port: Option<u16>,
    /// Token file to read that holds a previously created access token for the switches
    #[clap(long, default_value = "~/wasmer/token")]
    pub token_path: String,
    /// URL where the web data is remotely stored on a distributed commit log.
    #[clap(short, long, default_value = "ws://wasmer.sh/db")]
    pub db_url: url::Url,
    /// URL of the authentication servers
    #[clap(long, default_value = "ws://wasmer.sh/auth")]
    pub auth_url: url::Url,
    /// Domain name that has authority on instances
    #[clap(long, default_value = "wasmer.sh")]
    pub instance_authority: String,
    /// Ensures that this combined server(s) runs as a specific node_id
    #[clap(short, long)]
    pub node_id: Option<u32>,
    /// Optional list of the nodes that make up this cluster
    #[clap(long)]
    pub nodes_list: Option<String>,
    /// Time-to-live for virtual switches remain active
    #[clap(long, default_value = "300")]
    pub ttl: u64,
}