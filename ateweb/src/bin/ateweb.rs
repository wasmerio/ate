use std::time::Duration;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use clap::Parser;

use ate::prelude::*;
use ate_auth::helper::*;
use ateweb::opt::*;
use ateweb::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();

    ate::log_init(opts.verbose, opts.debug);

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

    // Run the server
    match opts.subcmd {
        SubCommand::Web(run) => {
            let web_key: EncryptKey = load_key(run.web_key_path.clone(), ".read");

            conf.log_path = Some(run.log_path);
            let server = ServerBuilder::new(run.remote, run.auth_url)
                .with_web_master_key(web_key)
                .with_conf(&conf)
                .ttl(Duration::from_secs(run.ttl))
                .add_listener(run.listen, run.port, run.port == 443u16)
                .build()
                .await?;
            TaskEngine::run_until(server.run()).await?;
        }

        SubCommand::All(run) => {
            let web_key: EncryptKey = load_key(run.web_key_path.clone(), ".read");
            let protocol = StreamProtocol::parse(&run.auth_url)?;
            let port = run.auth_url.port().unwrap_or(protocol.default_port());
            let domain = run.auth_url.domain().unwrap_or("localhost").to_string();

            let mut cfg_mesh = ConfMesh::skeleton(&conf, domain, port, Some(0u32)).await?;
            cfg_mesh.wire_protocol = protocol;
            cfg_mesh.wire_encryption = Some(KeySize::Bit192);
            cfg_mesh.listen_certificate = Some(PrivateEncryptKey::generate(KeySize::Bit192));
            let root = create_server(&cfg_mesh).await?;

            let mut router = ate::comms::StreamRouter::new(
                cfg_mesh.wire_format.clone(),
                cfg_mesh.wire_protocol.clone(),
                cfg_mesh.listen_certificate.clone(),
                root.server_id(),
                cfg_mesh.accept_timeout,
            );
            router.set_default_route(root);

            conf.log_path = Some(run.log_path);
            let server = ServerBuilder::new(run.remote, run.auth_url)
                .with_web_master_key(web_key)
                .with_conf(&conf)
                .ttl(Duration::from_secs(run.ttl))
                .with_callback(router)
                .add_listener(run.listen, run.port, run.port == 443u16)
                .build()
                .await?;
            TaskEngine::run_until(server.run()).await?;
        }
    }

    info!("ateweb::shutdown");

    Ok(())
}
