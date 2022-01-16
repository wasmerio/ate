#[allow(unused_imports, dead_code)]
use tracing::{info, error, debug};
use ate::{prelude::*};
use clap::Parser;

/// Runs the session server
#[derive(Parser)]
pub struct OptsSessionServer {
    /// IP address that the datachain server will isten on
    #[clap(short, long, default_value = "::")]
    pub listen: IpAddr,
    /// Port that the server will listen on for HTTP requests which are then turned into websocket
    #[clap(long)]
    pub port: Option<u16>,
    /// Forces Tokera to listen on a specific port for HTTPS requests with generated certificates
    #[clap(long)]
    pub tls_port: Option<u16>,
    /// Path to the secret key that grants access to the EdgeServer role within groups
    #[clap(long, default_value = "~/ate/session.key")]
    pub session_key_path: String,
    /// URL where the web data is remotely stored on a distributed commit log.
    #[clap(short, long, default_value = "ws://tokera.com/db")]
    pub db_url: url::Url,
    /// URL of the authentication servers
    #[clap(long, default_value = "ws://tokera.com/auth")]
    pub auth_url: url::Url,
}