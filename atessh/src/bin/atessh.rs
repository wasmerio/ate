use clap::Parser;
use tokio::sync::watch;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use atessh::key::*;
use atessh::opt::*;
use atessh::server::Server;
use atessh::term_lib;
use atessh::utils::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();

    // Enable the logging
    log_init(opts.verbose, opts.debug);

    // Set the system
    let (tx_exit, _rx_exit) = watch::channel(false);
    let sys = tokterm::system::SysSystem::new(tx_exit);
    term_lib::api::set_system_abi(sys.clone());

    // Process the command
    match opts.subcmd {
        SubCommand::Ssh(run) => {
            sys.block_on(async move {
                // Load the SSH key
                let server_key: SshServerKey = load_key(run.key_path.clone());

                // Start the SSH server
                let server = Server::new(run, server_key).await;
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
