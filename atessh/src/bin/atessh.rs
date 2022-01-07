use clap::Parser;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use atessh::key::*;
use atessh::opt::*;
use atessh::server::Server;
use atessh::utils::*;
use std::sync::Arc;
use tokio::runtime::Builder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();

    // Enable the logging
    log_init(opts.verbose, opts.debug);

    // Create the runtime
    let runtime = Arc::new(Builder::new_multi_thread().enable_all().build().unwrap());

    // Process the command
    match opts.subcmd {
        SubCommand::Ssh(run) => {
            runtime.clone().block_on(async move {
                // Load the SSH key
                let server_key: SshServerKey = load_key(run.key_path.clone());

                // Start the SSH server
                let server = Server::new(run, server_key, runtime).await;
                server.listen().await?;
                Ok(())
            })
        }
        SubCommand::Generate(generate) => {
            let key = SshServerKey::generate_ed25519();
            save_key(generate.key_path.clone(), key);
            Ok(())
        }
    }
}
