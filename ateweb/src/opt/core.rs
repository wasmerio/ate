use std::net::IpAddr;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use url::Url;

use clap::Parser;

#[derive(Parser)]
#[clap(version = "1.6", author = "John S. <johnathan.sharratt@gmail.com>")]
pub struct Opts {
    /// Sets the level of log verbosity, can be used multiple times
    #[allow(dead_code)]
    #[clap(short, long, parse(from_occurrences))]
    pub verbose: i32,
    /// URL where the user is authenticated
    #[clap(short, long, default_value = "ws://tokera.com/auth")]
    pub auth: Url,
    /// No NTP server will be used to synchronize the time thus the server time
    /// will be used instead
    #[clap(long)]
    pub no_ntp: bool,
    /// NTP server address that the file-system will synchronize with
    #[clap(long)]
    pub ntp_pool: Option<String>,
    /// NTP server port that the file-system will synchronize with
    #[clap(long)]
    pub ntp_port: Option<u16>,
    /// Logs debug info to the console
    #[clap(short, long)]
    pub debug: bool,
    /// Determines if ATE will use DNSSec or just plain DNS
    #[clap(long)]
    pub dns_sec: bool,
    /// Address that DNS queries will be sent to
    #[clap(long, default_value = "8.8.8.8")]
    pub dns_server: String,
    /// Token file to read that holds a previously created token to be used for this operation
    #[clap(long, default_value = "~/ate/token")]
    pub token_path: String,

    #[clap(subcommand)]
    pub subcmd: SubCommand,
}

/// Runs a web server that will serve content from a Tokera file system
#[derive(Parser)]
pub struct OptsWeb {
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
    /// URL where the authentication requests will be lodged.
    #[clap(short, long, default_value = "ws://tokera.com/auth")]
    pub auth_url: Url,
    /// Path to the secret key that grants access to the WebServer role within groups
    #[clap(long, default_value = "~/ate/web.key")]
    pub web_key_path: String,
    /// Location where all the websites will be cached
    #[clap(long, default_value = "/tmp/www")]
    pub log_path: String,
}

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
    /// Location where all the websites will be cached
    #[clap(long, default_value = "/tmp/www")]
    pub log_path: String,
    /// Path to the secret key that helps protect key operations like creating users and resetting passwords
    #[clap(long, default_value = "~/ate/auth.key")]
    pub auth_key_path: String,
    /// Path to the secret key that grants access to the WebServer role within groups
    #[clap(long, default_value = "~/ate/web.key")]
    pub web_key_path: String,
    /// Path to the log files where all the authentication data is stored
    #[clap(long, default_value = "~/ate/auth")]
    pub auth_logs_path: String,
    /// URL where the data is remotely stored on a distributed commit log.
    #[clap(short, long, default_value = "ws://tokera.com/db")]
    pub remote: Url,
    /// Address that the authentication server(s) are listening and that
    /// this server can connect to if the chain is on another mesh node
    #[clap(short, long, default_value = "ws://localhost:5001/auth")]
    pub auth_url: url::Url,
}

#[derive(Parser)]
pub enum SubCommand {
    /// Starts a web server that will load Tokera file systems and serve
    /// them directly as HTML content
    #[clap()]
    Web(OptsWeb),
    /// Starts a web server that will load Tokera file systems and serve
    /// them directly as HTML content along with a database engine and
    /// authentication server
    #[clap()]
    All(OptsAll),
}
