use clap::Parser;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use tokio::sync::watch;
use ate_auth::helper::conf_cmd;

use atessh::key::*;
use atessh::opt::*;
use atessh::server::Server;
use atessh::utils::*;
use std::sync::Arc;
use tokio::runtime::Builder;
use atessh::term_lib;
use term_lib::bin_factory::CachedCompiledModules;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();

    // Enable the logging
    log_init(opts.verbose, opts.debug);

    // Create the runtime
    let runtime = Arc::new(Builder::new_multi_thread().enable_all().build().unwrap());

    // Process the command
    let key_path = opts.key_path.clone();
    match opts.subcmd {
        SubCommand::Ssh(ssh) => {
            match ssh.action {
                OptsSshAction::Host(host) => {
                    runtime.clone().block_on(async move {
                        // Load the SSH key
                        let server_key: SshServerKey = load_key(key_path);

                        // Create the registry that will be used to validate logins
                        let registry = ate::mesh::Registry::new(&conf_cmd()).await.cement();

                        // Set the system
                        let (tx_exit, rx_exit) = watch::channel(false);
                        let sys = Arc::new(tokterm::system::SysSystem::new_with_runtime(
                            tx_exit, runtime,
                        ));

                        // Start the system and add the native files
                        let sys = atessh::system::System::new(sys, registry.clone(), host.db_url.clone(), host.native_files.clone()).await;
                        let native_files = sys.native_files.clone();
                        term_lib::api::set_system_abi(sys);

                        // Start the SSH server
                        let compiled_modules = Arc::new(CachedCompiledModules::new(Some(host.compiler_cache_path.clone())));
                        let server = Server::new(host, server_key, registry, compiled_modules, native_files, rx_exit).await;
                        server.listen().await?;
                        Ok(())
                    })
                }
                OptsSshAction::Generate(_) => {
                    let key = SshServerKey::generate_ed25519();
                    save_key(key_path, key);
                    Ok(())
                }
            }
        }
    }
}
