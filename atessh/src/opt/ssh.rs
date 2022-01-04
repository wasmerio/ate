use std::net::IpAddr;
use tokterm::term_lib;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use clap::Parser;

/// Runs a ssh server
#[derive(Parser)]
pub struct OptsSsh {
    /// IP address that the SSH server will isten on
    #[clap(short, long, default_value = "::")]
    pub listen: IpAddr,
    /// Port that the server will listen on for SSH requests
    #[clap(long, default_value = "22")]
    pub port: u16,
    /// Path to the secret server key
    #[clap(default_value = "~/ate/ssh.server.key")]
    pub key_path: String,
    /// URL where the user is authenticated
    #[clap(short, long, default_value = "ws://tokera.com/auth")]
    pub auth: url::Url,
    /// Determines which compiler to use
    #[clap(short, long, default_value = "default")]
    pub compiler: term_lib::eval::Compiler,
}
