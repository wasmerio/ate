#[allow(unused_imports, dead_code)]
use tracing::{info, error, debug};
use ate::prelude::*;
use clap::Parser;

use super::*;

#[derive(Parser)]
#[clap(version = "1.0", author = "Wasmer Inc <info@wasmer.io>")]
pub struct Opts {
    /// Sets the level of log verbosity, can be used multiple times
    #[allow(dead_code)]
    #[clap(short, long, parse(from_occurrences))]
    pub verbose: i32,
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
    #[clap(long, default_value = "~/wasmer/token")]
    pub token_path: String,
    /// Path to the certificate file that will be used by an listening servers
    /// (there must be TXT records in the host domain servers for this cert)
    #[clap(long, default_value = "~/wasmer/cert")]
    pub cert_path: String,
    /// Path to the secret server key
    #[clap(default_value = "~/wasmer/ssh.server.key")]
    pub ssh_key_path: String,
    /// Indicates if ATE will use quantum resistant wire encryption (possible values
    /// are 128, 192, 256). When running in 'centralized' mode wire encryption will
    /// default to 128bit however when running in 'distributed' mode wire encryption
    /// will default to off unless explicitly turned on.
    #[clap(long, default_value = "128")]
    pub wire_encryption: KeySize,

    #[clap(subcommand)]
    pub subcmd: SubCommand,
}

#[derive(Parser)]
pub enum SubCommand {
    /// Hosts the network server
    #[clap()]
    Run(OptsNetworkServer),
}