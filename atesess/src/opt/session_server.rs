#[allow(unused_imports, dead_code)]
use tracing::{info, error, debug};
use ate::{prelude::*};
use clap::Parser;
use atessh::term_lib;

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
    /// Token file to read that holds a previously created token to be used for this operation
    #[clap(long, default_value = "~/ate/token")]
    pub token_path: String,
    /// URL where the web data is remotely stored on a distributed commit log.
    #[clap(short, long, default_value = "ws://tokera.com/db")]
    pub db_url: url::Url,
    /// URL of the authentication servers
    #[clap(long, default_value = "ws://tokera.com/auth")]
    pub auth_url: url::Url,
    /// URL of the session servers that clients will connect to
    #[clap(long, default_value = "ws://tokera.com/sess")]
    pub sess_url: url::Url,
    /// Ensures that this combined server(s) runs as a specific node_id
    #[clap(short, long)]
    pub node_id: Option<u32>,
    /// Location where the native binary files are stored
    #[clap(long, default_value = "tokera.sh/www")]
    pub native_files: String,
    /// Instance authority that has the access rights to run service instances.
    #[clap(long, default_value = "tokera.sh")]
    pub instance_authority: String,
    /// Determines which compiler to use
    #[clap(short, long, default_value = "default")]
    pub compiler: term_lib::eval::Compiler,
}