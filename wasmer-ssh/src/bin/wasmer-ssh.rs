use clap::Parser;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use wasmer_wasi::bin_factory::CachedCompiledModules;
use wasmer_ssh::key::*;
use wasmer_ssh::opt::*;
use wasmer_ssh::server::Server;
use wasmer_ssh::utils::*;
use std::sync::Arc;
use tokio::runtime::Builder;

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

                        // Start the SSH server
                        let compiled_modules = Arc::new(
                            CachedCompiledModules::new(
                                Some(host.compiler_cache_path.clone()),
                                Some(host.webc_dir.clone())
                            )
                        );
                        let server = Server::new(host, server_key, compiled_modules).await;
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
