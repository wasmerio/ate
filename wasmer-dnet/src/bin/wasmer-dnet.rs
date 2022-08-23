#![recursion_limit="256"]
#![allow(unused_imports)]
use ate::mesh::MeshHashTable;
use ate::utils::load_node_list;
use wasmer_dnet::common::setup_server;
use wasmer_dnet::server::Server;
use tokio::sync::watch;
#[allow(unused_imports, dead_code)]
use tracing::{info, error, debug, trace, warn};
use ate::prelude::*;
use std::sync::Arc;
use clap::Parser;
use tokio::sync::watch::Receiver;
use tokio::select;
use tokio::runtime::Builder;

use ate::comms::StreamRouter;

use wasmer_auth::helper::try_load_key;

use wasmer_dnet::opt::*;

#[allow(dead_code)]
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
    conf.record_type_name = true;
    
    let ret = runtime.clone().block_on(async move {
        match opts.subcmd {
            SubCommand::Run(solo) => {
                conf.nodes = load_node_list(solo.nodes_list.clone());
                let (_server, hard_exit) = setup_server(
                    solo,
                    conf,
                    Some(opts.wire_encryption),
                    Some(cert)
                ).await?;
                
                main_loop(Some(hard_exit)).await?;
                //server.shutdown().await;
            },
        }
        Ok(())
    });

    println!("Goodbye!");
    ret
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
