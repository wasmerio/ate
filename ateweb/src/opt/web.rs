use std::net::IpAddr;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use url::Url;

use clap::Parser;

/// Runs a web server that will serve content from a Tokera file system
#[derive(Parser)]
pub struct OptsWeb {
    /// IP address that the web server will isten on
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
}
