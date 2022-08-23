use std::time::Duration;
#[cfg(feature = "ate")]
use ate::utils::load_node_list;
#[cfg(feature = "wasmer-auth")]
use wasmer_auth::flow::ChainFlow;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use clap::Parser;

#[cfg(feature = "ate")]
use ate::prelude::*;
#[cfg(feature = "wasmer-auth")]
use wasmer_auth::helper::*;
use wasmer_gw::opt::*;
use wasmer_gw::*;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();

    #[cfg(feature = "ate")]
    ate::log_init(opts.verbose, opts.debug);
    #[cfg(not(feature = "ate"))]
    tracing_subscriber::fmt::SubscriberBuilder::default()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    #[cfg(feature = "ate")]
    #[allow(unused_mut)]
    let mut conf = {
        let mut conf = AteConfig::default();
        conf.dns_sec = opts.dns_sec;
        conf.dns_server = opts.dns_server;
        conf.ntp_sync = opts.no_ntp == false;

        if let Some(pool) = opts.ntp_pool {
            conf.ntp_pool = pool;
        }
        if let Some(port) = opts.ntp_port {
            conf.ntp_port = port;
        }
        conf
    };

    // Run the server
    match opts.subcmd {
        #[cfg(feature = "wasmer-auth")]
        SubCommand::Auth(run) => {
            conf.nodes = load_node_list(run.nodes_list);

            // Open the key file
            let root_write_key: PrivateSignKey = load_key(run.auth_key_path.clone(), ".write");
            let root_read_key: EncryptKey = load_key(run.auth_key_path.clone(), ".read");
            let root_cert_key: PrivateEncryptKey = load_key(run.cert_path.clone(), "");
            let web_key: EncryptKey = load_key(run.web_key_path.clone(), ".read");
            let edge_key: EncryptKey = load_key(run.edge_key_path.clone(), ".read");
            let contract_key: EncryptKey = load_key(run.contract_key_path.clone(), ".read");
            
            let mut session = AteSessionUser::new();
            session.user.add_read_key(&root_read_key);
            session.user.add_write_key(&root_write_key);

            // Create the server and listen
            let mut flow = ChainFlow::new(
                &conf,
                root_write_key,
                session,
                web_key,
                edge_key,
                contract_key,
                &run.url,
            );
            flow.terms_and_conditions = Some(wasmer_auth::GENERIC_TERMS_AND_CONDITIONS.to_string());
            let mut cfg_mesh =
                ConfMesh::solo_from_url(&conf, &run.url, &run.listen, None, run.node_id).await?;
            cfg_mesh.wire_protocol = StreamProtocol::parse(&run.url)?;
            cfg_mesh.listen_certificate = Some(root_cert_key);

            let server = create_server(&cfg_mesh).await?;
            server.add_route(Box::new(flow), &conf).await?;

            // Wait for ctrl-c
            let mut exit = ctrl_channel();
            while *exit.borrow() == false {
                exit.changed().await.unwrap();
            }
            println!("Shutting down...");
            server.shutdown().await;
            println!("Goodbye!");
        }

        SubCommand::Web(run) => {
            #[cfg(feature = "dfs")]
            let web_key: EncryptKey = load_key(run.web_key_path.clone(), ".read");

            #[allow(unused_variables)]
            let is_tls = run.port == 443u16;
            #[cfg(feature = "dfs")]
            {
                conf.log_path = Some(run.log_path);
            }
            
            #[allow(unused_mut)]
            let mut server = ServerBuilder::new(
                #[cfg(feature = "dfs")]
                run.remote,
                #[cfg(feature = "dfs")]
                run.auth_url
            )
            .with_www_path(run.www_path)
            .ttl(Duration::from_secs(run.ttl))
            .add_listener(
                run.listen,
                run.port,
                #[cfg(feature = "tls")]
                is_tls
            );
            
            #[cfg(feature = "dfs")]
            {
                server = server
                    .with_web_master_key(web_key)
                    .with_conf(&conf);
            }
            
            let server = server
                .build()
                .await?;
            server.run().await?;
        }

        SubCommand::All(run) => {
            #[allow(unused_variables)]
            let is_tls = run.port == 443u16;
            #[cfg(feature = "ate")]
            {
                conf.log_path = Some(run.log_path);
            }

            #[allow(unused_mut)]
            let mut server = ServerBuilder::new(
                #[cfg(feature = "dfs")]
                run.remote,
                #[cfg(feature = "dfs")]
                run.auth_url.clone()
            )
            .with_www_path(run.www_path)
            .ttl(Duration::from_secs(run.ttl))
            .add_listener(
                run.listen,
                run.port,
                #[cfg(feature = "tls")]
                is_tls
            );

            #[cfg(feature = "dfs")]
            {
                conf.nodes = load_node_list(run.nodes_list);

                // Open the key file
                let root_write_key: PrivateSignKey = load_key(run.auth_key_path.clone(), ".write");
                let root_read_key: EncryptKey = load_key(run.auth_key_path.clone(), ".read");
                let root_cert_key: PrivateEncryptKey = load_key(run.cert_path.clone(), "");
                let web_key: EncryptKey = load_key(run.web_key_path.clone(), ".read");
                let edge_key: EncryptKey = load_key(run.edge_key_path.clone(), ".read");
                let contract_key: EncryptKey = load_key(run.contract_key_path.clone(), ".read");
                
                let mut session = AteSessionUser::new();
                session.user.add_read_key(&root_read_key);
                session.user.add_write_key(&root_write_key);

                let protocol = StreamProtocol::parse(&run.auth_url)?;
                let port = run.auth_url.port().unwrap_or(protocol.default_port());
                let domain = run.auth_url.domain().unwrap_or("localhost").to_string();

                let mut cfg_mesh = ConfMesh::skeleton(&conf, domain, port, Some(0u32)).await?;
                cfg_mesh.wire_protocol = protocol;
                cfg_mesh.wire_encryption = Some(KeySize::Bit192);
                cfg_mesh.listen_certificate = Some(root_cert_key);
                let root = create_server(&cfg_mesh).await?;

                // Create the server and listen
                let mut flow = ChainFlow::new(
                    &conf,
                    root_write_key,
                    session,
                    web_key,
                    edge_key,
                    contract_key,
                    &run.auth_url,
                );
                flow.terms_and_conditions = Some(wasmer_auth::GENERIC_TERMS_AND_CONDITIONS.to_string());

                root.add_route(Box::new(flow), &conf).await?;

                let mut router = ate::comms::StreamRouter::new(
                    cfg_mesh.wire_format.clone(),
                    cfg_mesh.wire_protocol.clone(),
                    None,
                    cfg_mesh.listen_certificate.clone(),
                    root.server_id(),
                    cfg_mesh.accept_timeout,
                );
                router.set_default_route(root);

                server = server
                    .with_web_master_key(web_key)
                    .with_conf(&conf)
                    .with_callback(router);
            }

            let server = server.build()
                .await?;

            server.run().await?;
        }
    }

    info!("wasmer_gw::shutdown");

    Ok(())
}

#[allow(dead_code)]
fn ctrl_channel() -> tokio::sync::watch::Receiver<bool> {
    let (sender, receiver) = tokio::sync::watch::channel(false);
    ctrlc_async::set_handler(move || {
        let _ = sender.send(true);
    })
    .unwrap();
    receiver
}