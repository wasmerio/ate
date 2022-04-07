use std::net::IpAddr;
use tokterm::term_lib;
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
    pub compiler: term_lib::eval::Compiler,
    /// Location where cached compiled modules are stored
    #[clap(long, default_value = "~/ate/compiled")]
    pub compiler_cache_path: String,
    /// URL of the datachain servers (e.g. wss://tokera.sh/db)
    #[clap(long)]
    pub db_url: Option<url::Url>,
    /// URL of the authentication servers (e.g. wss://tokera.sh/auth)
    #[clap(long)]
    pub auth_url: Option<url::Url>,
    /// Location where the native binary files are stored
    #[clap(long, default_value = "tokera.sh/www")]
    pub native_files: String,
}
