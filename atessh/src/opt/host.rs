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
    /// URL of the datachain servers
    #[clap(long, default_value = "ws://tokera.com/db")]
    pub db_url: url::Url,
    /// URL of the authentication servers
    #[clap(long, default_value = "ws://tokera.com/auth")]
    pub auth_url: url::Url,
    /// Location where the native binary files are stored
    #[clap(long, default_value = "tokera.sh/www")]
    pub native_files: String,
}
