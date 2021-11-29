#![allow(unused_imports)]
use clap::Parser;
use tracing::{debug, error, info, warn};

#[allow(dead_code)]
#[derive(Parser)]
#[clap(version = "1.0", author = "Tokera Pty Ltd <info@tokera.com>")]
struct Opts {
    /// Sets the level of log verbosity, can be used multiple times
    #[clap(short, long, parse(from_occurrences))]
    pub verbose: i32,
    /// Logs debug info to the console
    #[clap(short, long)]
    pub debug: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = Opts::parse();
    tokterm::utils::log_init(opts.verbose, opts.debug);

    // We are done
    Ok(())
}
