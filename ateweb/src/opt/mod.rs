#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use url::Url;

use clap::Clap;

#[derive(Clap)]
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

    #[clap(subcommand)]
    pub subcmd: SubCommand,
}

/// Runs a web server that will serve content from a Tokera file system
#[derive(Clap)]
pub struct OptsRun {
    /// URL where the data is remotely stored on a distributed commit log.
    #[clap(short, long, default_value = "ws://tokera.com/db")]
    pub remote: Url,
    /// (Optional) Location of the local persistent redo log (e.g. ~/ate/fs")
    /// If this parameter is not specified then chain-of-trust will cache in memory rather than disk
    #[clap(long)]
    pub log_path: Option<String>,
}

#[derive(Clap)]
pub enum SubCommand {
    /// Starts a web server that will load Tokera file systems and serve
    /// them directly as HTML content
    #[clap()]
    Run(OptsRun),
}