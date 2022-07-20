#[allow(unused_imports, dead_code)]
use tracing::{info, error, debug};
use ate::{prelude::*};
use clap::Parser;
use wasmer_ssh::wasmer_os;

/// Runs the session server
#[derive(Parser)]
pub struct OptsSessionServer {
    /// Optional list of the nodes that make up this cluster
    #[clap(long)]
    pub nodes_list: Option<String>,
    /// IP address that the datachain server will isten on
    #[clap(short, long, default_value = "::")]
    pub listen: IpAddr,
    /// Port that the server will listen on for HTTP requests which are then turned into websocket
    #[clap(long)]
    pub port: Option<u16>,
    /// Forces Wasmer to listen on a specific port for HTTPS requests with generated certificates
    #[clap(long)]
    pub tls_port: Option<u16>,
    /// Token file to read that holds a previously created token to be used for this operation
    #[clap(long, default_value = "~/wasmer/token")]
    pub token_path: String,
    /// Location where cached compiled modules are stored
    #[clap(long, default_value = "~/wasmer/compiled")]
    pub compiler_cache_path: String,
    /// URL where the web data is remotely stored on a distributed commit log.
    #[clap(short, long, default_value = "ws://wasmer.sh/db")]
    pub db_url: url::Url,
    /// URL of the authentication servers
    #[clap(long, default_value = "ws://wasmer.sh/auth")]
    pub auth_url: url::Url,
    /// URL of the session servers that clients will connect to
    #[clap(long, default_value = "ws://wasmer.sh/inst")]
    pub inst_url: url::Url,
    /// Ensures that this combined server(s) runs as a specific node_id
    #[clap(short, long)]
    pub node_id: Option<u32>,
    /// Location where the native binary files are stored
    #[clap(long, default_value = "wasmer.sh/www")]
    pub native_files: String,
    /// Uses a local directory for native files rather than the published ate chain
    #[clap(long)]
    pub native_files_path: Option<String>,
    /// Determines which compiler to use
    #[clap(short, long, default_value = "default")]
    pub compiler: wasmer_os::eval::Compiler,
    /// Time-to-live for sessions that are initiated
    #[clap(long, default_value = "300")]
    pub ttl: u64,
}