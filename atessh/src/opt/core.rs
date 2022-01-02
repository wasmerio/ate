use clap::Parser;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use super::generate::OptsGenerate;
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

    #[clap(subcommand)]
    pub subcmd: SubCommand,
}

#[derive(Parser)]
pub enum SubCommand {
    /// Starts a ssh server
    #[clap()]
    Ssh(OptsSsh),
    /// Generates the SSH serve side keys
    #[clap()]
    Generate(OptsGenerate),
}
