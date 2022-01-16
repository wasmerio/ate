#![recursion_limit="256"]
#[allow(unused_imports, dead_code)]
use tracing::{info, error, debug, trace, warn};
use ate::prelude::*;
use std::sync::Arc;
use clap::Parser;
use tokio::sync::watch::Receiver;
use tokio::select;
use tokio::runtime::Builder;

use ate::mesh::MeshRoot;

use ate_auth::helper::load_key;
use ate_auth::helper::try_load_key;
use ate_auth::helper::save_key;

use atesess::opt::*;

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

    if let SubCommand::Generate(generate) = &opts.subcmd {
        let read_key = EncryptKey::generate(generate.strength);
        println!("read-key-hash: {}", read_key.hash());
        save_key(generate.key_path.clone(), read_key, ".read");            

        let write_key = PrivateSignKey::generate(generate.strength);
        println!("write-key-hash: {}", write_key.hash());
        save_key(generate.key_path.clone(), write_key, ".write");

        let cert_key = PrivateEncryptKey::generate(generate.strength);
        println!("cert-key-hash: {}", cert_key.hash());
        println!("TXT:    ate-cert-{}", cert_key.hash().to_hex_string().to_lowercase());
        save_key(generate.key_path.clone(), cert_key, ".cert");
        return Ok(());
    }
        
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
            SubCommand::Generate(_) => {
                // this was already done earlier
            },
            SubCommand::SessionServer(solo) => {
                let (_server, hard_exit) = main_web(&solo, conf, None).await?;
                
                main_loop(Some(hard_exit)).await?;
                //server.shutdown().await;
            },
        }
        Ok(())
    });

    println!("Goodbye!");
    ret
}

async fn main_web(solo: &OptsSessionServer, cfg_ate: ConfAte, callback: Option<&Arc<MeshRoot>>) -> Result<(Arc<ateweb::server::Server>, Receiver<bool>), AteError>
{
    let web_key: EncryptKey = load_key(solo.session_key_path.clone(), ".read");
    let mut builder = ateweb::builder::ServerBuilder::new(solo.db_url.clone(), solo.auth_url.clone(), web_key)
        .add_listener(solo.listen, solo.port.unwrap_or(80u16), false)
        .add_listener(solo.listen, solo.tls_port.unwrap_or(443u16), true)
        .with_conf(&cfg_ate);

    if let Some(callback) = callback {
        builder = builder
            .with_callback(ateweb::ServerMeshAdapter::new(callback));
    }

    let server = builder
        .build()
        .await?;

    let (hard_exit_tx, hard_exit_rx) = tokio::sync::watch::channel(false);
    
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
    Ok((server, hard_exit_rx))
}

async fn main_loop(hard_exit: Option<Receiver<bool>>) -> Result<(), Box<dyn std::error::Error>>
{
    TaskEngine::run_until(__main_loop(hard_exit)).await
}

async fn __main_loop(mut hard_exit: Option<Receiver<bool>>) -> Result<(), Box<dyn std::error::Error>>
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