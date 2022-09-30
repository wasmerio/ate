use std::net::IpAddr;
use wasmer_term::wasmer_os;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use clap::Parser;

/// Runs a ssh server host
#[derive(Parser)]
pub struct OptsHost {
    /// IP address that the SSH server will isten on
    #[clap(short, long, default_value = "::")]
    pub listen: IpAddr,
    /// Port that the server will listen on for SSH requests
    #[clap(long, default_value = "22")]
    pub port: u16,
    /// Determines which compiler to use
    #[clap(short, long, default_value = "default")]
    pub compiler: wasmer_os::eval::Compiler,
    /// Location where cached compiled modules are stored
    #[clap(long, default_value = "~/wasmer/compiled")]
    pub compiler_cache_path: String,
    /// URL of the datachain servers (e.g. wss://wasmer.sh/db)
    #[clap(long)]
    pub db_url: Option<url::Url>,
    /// URL of the authentication servers (e.g. wss://wasmer.sh/auth)
    #[clap(long)]
    pub auth_url: Option<url::Url>,
    /// Location where the native binary files are stored
    #[clap(long, default_value = "wasmer.sh/www")]
    pub native_files: String,
    /// Uses a local directory for native files rather than the published ate chain
    #[clap(long)]
    pub native_files_path: Option<String>,
}
