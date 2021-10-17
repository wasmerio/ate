#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use url::Url;
use std::net::IpAddr;

use clap::Parser;

/// Runs a web server that will serve content from a Tokera file system
#[derive(Parser)]
pub struct OptsAll {
    /// IP address that the datachain server will isten on
    #[clap(short, long, default_value = "::")]
    pub listen: IpAddr,
    /// Port that the server will listen on for HTTP requests
    #[clap(long, default_value = "80")]
    pub port: u16,
    /// Number of seconds that a website will remain idle in memory before it is evicted
    #[clap(long, default_value = "60")]
    pub ttl: u64,
    /// URL where the data is remotely stored on a distributed commit log.
    #[clap(short, long, default_value = "ws://tokera.com/db")]
    pub remote: Url,
    /// Location where all the websites will be cached
    #[clap(long, default_value = "/tmp/www")]
    pub log_path: String,
    /// Path to the secret key that helps protect key operations like creating users and resetting passwords
    #[clap(long, default_value = "~/ate/auth.key")]
    pub auth_key_path: String,
    /// Path to the log files where all the authentication data is stored
    #[clap(long, default_value = "~/ate/auth")]
    pub auth_logs_path: String,
    /// Address that the authentication server(s) are listening and that
    /// this server can connect to if the chain is on another mesh node
    #[clap(short, long, default_value = "ws://localhost:5001/auth")]
    pub auth_url: url::Url,
}