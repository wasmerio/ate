use clap::Parser;
use std::net::IpAddr;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

/// Runs the login authentication and authorization server
#[derive(Parser)]
pub struct OptsAuth {
    /// Path to the secret key that helps protect key operations like creating users and resetting passwords
    #[clap(long, default_value = "~/ate/auth.key")]
    pub auth_key_path: String,
    /// Path to the secret key that grants access to the WebServer role within groups
    #[clap(long, default_value = "~/ate/web.key")]
    pub web_key_path: String,
    /// Path to the secret key that grants access to the EdgeCompute role within groups
    #[clap(long, default_value = "~/ate/edge.key")]
    pub edge_key_path: String,
    /// Path to the secret key that grants access to the contracts
    #[clap(long, default_value = "~/ate/contract.key")]
    pub contract_key_path: String,
    /// Path to the log files where all the authentication data is stored
    #[clap(index = 1, default_value = "~/ate/auth")]
    pub logs_path: String,
    /// Path to the backup and restore location of log files
    #[clap(short, long)]
    pub backup_path: Option<String>,
    /// Address that the authentication server(s) are listening and that
    /// this server can connect to if the chain is on another mesh node
    #[clap(short, long, default_value = "ws://localhost:5001/auth")]
    pub url: url::Url,
    /// IP address that the authentication server will isten on
    #[clap(short, long, default_value = "::")]
    pub listen: IpAddr,
    /// Ensures that this authentication server runs as a specific node_id
    #[clap(short, long)]
    pub node_id: Option<u32>,
}