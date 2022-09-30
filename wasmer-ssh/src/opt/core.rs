use clap::Parser;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use super::ssh::OptsSsh;

#[derive(Parser)]
#[clap(version = "1.0", author = "John S. <johnathan.sharratt@gmail.com>")]
pub struct Opts {
    /// Sets the level of log verbosity, can be used multiple times
    #[allow(dead_code)]
    #[clap(short, long, parse(from_occurrences))]
    pub verbose: i32,
    /// Logs debug info to the console
    #[clap(short, long)]
    pub debug: bool,
    /// URL where the user is authenticated (e.g. ws://wasmer.sh/auth)
    #[clap(short, long)]
    pub auth: Option<url::Url>,
    /// Path to the secret server key
    #[clap(default_value = "~/wasmer/ssh.server.key")]
    pub key_path: String,

    #[clap(subcommand)]
    pub subcmd: SubCommand,
}

#[derive(Parser)]
pub enum SubCommand {
    /// Starts an SSH command
    #[clap()]
    Ssh(OptsSsh),
}
