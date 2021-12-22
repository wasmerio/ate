#![allow(unused_imports)]
use clap::Parser;
use term_lib::api::*;
use term_lib::console::Console;
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = Opts::parse();
    tokterm::utils::log_init(opts.verbose, opts.debug);

    // Set the system
    let sys = tokterm::system::SysSystem::new();
    term_lib::api::set_system_abi(sys.clone());
    let system = System::default();

    // Run the console
    system.fork_dedicated(move || async move {
        let location = "wss://localhost/".to_string();
        let user_agent = "noagent".to_string();
        let mut console = Console::new(location, user_agent);
        console.init().await;
    });

    // Block on the main system thread pool
    sys.run();

    // We are done
    Ok(())
}
