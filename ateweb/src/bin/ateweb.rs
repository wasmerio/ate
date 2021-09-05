#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};

use clap::Clap;

use ate::prelude::*;
use ateweb::opt::*;
use ateweb::builder::*;

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
    
    match opts.subcmd {
        SubCommand::Run(run) =>
        {
            let server = ServerBuilder::new(run.remote)
                .add_listener(run.listen, run.port_http, false, true)
                .add_listener(run.listen, run.port_https, true, true)
                .build().await;
            server.run().await?;
        }
    }

    info!("atefs::shutdown");

    Ok(())
}