#![recursion_limit="256"]
use ate::mesh::MeshHashTable;
use ate::utils::load_node_list;
use wasmer_instance::server::Server;
use tokio::sync::watch;
#[allow(unused_imports, dead_code)]
use tracing::{info, error, debug, trace, warn};
use ate::prelude::*;
use std::sync::Arc;
use clap::Parser;
use tokio::sync::watch::Receiver;
use tokio::select;
use tokio::runtime::Builder;
use wasmer_wasi::bin_factory::CachedCompiledModules;

use ate::comms::StreamRouter;

use wasmer_auth::helper::try_load_key;

use wasmer_instance::opt::*;

fn ctrl_channel() -> Receiver<bool> {
    let (sender, receiver) = tokio::sync::watch::channel(false);
    ctrlc::set_handler(move || {
        let _ = sender.send(true);
    }).unwrap();
    receiver
}

fn main() -> Result<(), Box<dyn std::error::Error>>
{
    // Create the runtime
    let runtime = Arc::new(Builder::new_multi_thread().enable_all().build().unwrap());

    let opts: Opts = Opts::parse();
    //let opts = main_debug();
    ate::log_init(opts.verbose, opts.debug);
        
    let cert: PrivateEncryptKey = match try_load_key(opts.cert_path.clone()) {
        Some(a) => a,
        None => {
            eprintln!("Failed to load the certificate ({}) - you must generate this first!", opts.cert_path);
            std::process::exit(1);
        }
    };
    ate::mesh::add_global_certificate(&cert.hash());

    let mut conf = AteConfig::default();
    conf.dns_sec = opts.dns_sec.clone();
    conf.dns_server = opts.dns_server.clone();
    
    let ret = runtime.clone().block_on(async move {
        match opts.subcmd {
            SubCommand::Run(solo) => {
                conf.nodes = load_node_list(solo.nodes_list);

                let protocol = StreamProtocol::parse(&solo.inst_url)?;
                let port = solo.auth_url.port().unwrap_or(protocol.default_port());
                let domain = solo.auth_url.domain().unwrap_or("localhost").to_string();
                let ttl = std::time::Duration::from_secs(solo.ttl);
                let nodes = load_node_list(solo.nodes_list);

                let mut cfg_mesh = ConfMesh::skeleton(&conf, domain, port, solo.node_id).await?;
                cfg_mesh.wire_protocol = protocol;
                cfg_mesh.wire_encryption = Some(opts.wire_encryption);
                cfg_mesh.listen_certificate = Some(cert);

                let table = MeshHashTable::new(&cfg_mesh);
                let server_id = table.compute_node_id(solo.node_id)?;

                let registry = Arc::new(Registry::new(&conf).await);

                let native_files = if let Some(path) = solo.native_files_path.clone() {
                    NativeFileType::LocalFileSystem(path)
                } else {
                    NativeFileType::AteFileSystem(solo.native_files.clone())
                };
        
                let mut instance_authority = solo.inst_url.domain()
                    .map(|a| a.to_string())
                    .unwrap_or_else(|| "wasmer.sh".to_string());
                if instance_authority == "localhost" {
                    instance_authority = "wasmer.sh".to_string();
                }

                let compiled_modules = Arc::new(CachedCompiledModules::new(Some(solo.compiler_cache_path.clone()), Some(solo.webc_dir)));
                let instance_server = Server::new(
                    solo.db_url.clone(),
                    solo.auth_url.clone(),
                    instance_authority.clone(),
                    solo.token_path.clone(),
                    registry.clone(),
                    compiled_modules.clone(),
                    ttl,
                ).await?;

                let mut router = ate::comms::StreamRouter::new(
                    cfg_mesh.wire_format.clone(),
                    cfg_mesh.wire_protocol.clone(),
                    cfg_mesh.listen_min_encryption.clone(),
                    cfg_mesh.listen_certificate.clone(),
                    server_id,
                    cfg_mesh.accept_timeout,
                );
                
                let route = Arc::new(instance_server);
                router.add_socket_route("/sess", route.clone()).await;
                router.add_socket_route("/inst", route.clone()).await;
                router.add_post_route("/sess", route.clone()).await;
                router.add_post_route("/inst", route.clone()).await;
                router.add_put_route("/sess", route.clone()).await;
                router.add_put_route("/inst", route.clone()).await;

                let (_server, hard_exit) = main_web(&solo, conf, Some(router)).await?;
                
                main_loop(Some(hard_exit)).await?;
                //server.shutdown().await;
            },
        }
        Ok(())
    });

    println!("Goodbye!");
    ret
}

#[allow(dead_code)]
async fn main_web(solo: &OptsSessionServer, cfg_ate: ConfAte, callback: Option<StreamRouter>) -> Result<(Arc<wasmer_gw::server::Server>, watch::Receiver<bool>), Box<dyn Error>>
{
    let (hard_exit_tx, hard_exit_rx) = tokio::sync::watch::channel(false);
    let server = main_web_ext(solo, cfg_ate, callback, hard_exit_tx).await?;
    Ok((server, hard_exit_rx))
}

async fn main_web_ext(solo: &OptsSessionServer, cfg_ate: ConfAte, callback: Option<StreamRouter>, hard_exit_tx: watch::Sender<bool>) -> Result<Arc<wasmer_gw::server::Server>, Box<dyn Error>>
{
    let mut builder = wasmer_gw::builder::ServerBuilder::new(
        solo.db_url.clone(),
        solo.auth_url.clone()
    )
        .add_listener(solo.listen, solo.port.unwrap_or(80u16), false)
        .add_listener(solo.listen, solo.tls_port.unwrap_or(443u16), true)
        .with_conf(&cfg_ate);

    if let Some(callback) = callback {
        builder = builder
            .with_callback(callback);
    }

    let server = builder
        .build()
        .await?;

    // Run the web server
    {
        let server = Arc::clone(&server);
        TaskEngine::spawn(async move {
            let ret = server.run().await;
            if let Err(err) = ret {
                error!("web server fatal error - {}", err);
            }
            let _ = hard_exit_tx.send(true);
        });
    }

    // Done
    Ok(server)
}

async fn main_loop(mut hard_exit: Option<Receiver<bool>>) -> Result<(), Box<dyn std::error::Error>>
{
    // Wait for ctrl-c
    eprintln!("Press ctrl-c to exit");
    let mut exit = ctrl_channel();
    while *exit.borrow() == false {
        match hard_exit.as_mut() {
            Some(hard_exit) => {
                select! {
                    a = exit.changed() => { a?; },
                    a = hard_exit.changed() => {
                        a?;
                        if *hard_exit.borrow() {
                            info!("Hard exit");
                            break;
                        }
                    }
                }
            },
            None => {
                exit.changed().await.unwrap();
            }
        }
    }
    println!("Shutting down...");
    Ok(())
}