#![allow(unused_imports, dead_code)]
use log::{info, error, debug};
use ate::{compact::CompactMode, prelude::*};
use std::env;
use std::io::ErrorKind;
use directories::BaseDirs;
use std::sync::Arc;
use std::ops::Deref;
use std::time::Duration;
use url::Url;
use tokio::select;

use clap::Clap;

mod flow;

use crate::flow::ChainFlow;

#[derive(Clap)]
#[clap(version = "1.4", author = "John S. <johnathan.sharratt@gmail.com>")]
struct Opts {
    /// Sets the level of log verbosity, can be used multiple times
    #[allow(dead_code)]
    #[clap(short, long, parse(from_occurrences))]
    verbose: i32,
    /// Logs debug info to the console
    #[clap(short, long)]
    debug: bool,
    /// URL where the user is authenticated
    #[clap(short, long, default_value = "ws://tokera.com/auth")]
    auth: Url,
    /// Indicates no authentication server will be used meaning all new chains
    /// created by clients allow anyone to write new root nodes.
    #[clap(long)]
    no_auth: bool,
    /// Indicates if ATE will use quantum resistant wire encryption (possible values
    /// are 128, 192, 256). When running in 'centralized' mode wire encryption will
    /// default to 128bit however when running in 'distributed' mode wire encryption
    /// will default to off unless explicitly turned on.
    #[clap(long)]
    wire_encryption: Option<KeySize>,
    /// Disbles wire encryption which would otherwise be turned on when running in 'centralized' mode.
    #[clap(long)]
    no_wire_encryption: bool,
    /// Trust mode that the database server will run under - valid values are either
    /// 'distributed' or 'centralized'. When running in 'distributed' mode the
    /// server itself does not need to be trusted in order to trust the data it holds
    /// however it has a significant performance impact on write operations while the
    /// 'centralized' mode gives much higher performance but the server needs to be
    /// protected.
    #[clap(short, long, default_value = "centralized")]
    trust: TrustMode,
    /// Determines if ATE will use DNSSec or just plain DNS
    #[clap(long)]
    dns_sec: bool,
    /// Address that DNS queries will be sent to
    #[clap(long, default_value = "8.8.8.8")]
    dns_server: String,

    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    #[clap()]
    Solo(Solo),
}
/// Runs a solo ATE database and listens for connections from clients
#[derive(Clap)]
struct Solo {
    /// Path to the log files where all the file system data is stored
    #[clap(index = 1, default_value = "/opt/ate")]
    logs_path: String,
    /// Address that the database server(s) are listening and that
    /// this server can connect to if the chain is on another mesh node
    #[clap(short, long, default_value = "ws://localhost:5000/db")]
    url: url::Url,
    /// IP address that the database server will isten on
    #[clap(short, long, default_value = "::")]
    listen: IpAddr,
    /// Mode that the compaction will run under (valid modes are 'never', 'modified', 'timer', 'factor', 'size', 'factor-or-timer', 'size-or-timer')
    #[clap(long, default_value = "factor-or-timer")]
    compact_mode: CompactMode,
    /// Time in seconds between compactions of the log file (default: 1 hour) - this argument is ignored if you select a compact_mode that has no timer
    #[clap(long, default_value = "3600")]
    compact_timer: u64,
    /// Factor growth in the log file which will trigger compaction - this argument is ignored if you select a compact_mode that has no growth trigger
    #[clap(long, default_value = "0.4")]
    compact_threshold_factor: f32,
    /// Size of growth in bytes in the log file which will trigger compaction (default: 100MB) - this argument is ignored if you select a compact_mode that has no growth trigger
    #[clap(long, default_value = "104857600")]
    compact_threshold_size: u64,
}

fn ctrl_channel() -> tokio::sync::watch::Receiver<bool> {
    let (sender, receiver) = tokio::sync::watch::channel(false);
    ctrlc::set_handler(move || {
        let _ = sender.send(true);
    }).unwrap();
    receiver
}

#[cfg_attr(feature = "enable_mt", tokio::main(flavor = "multi_thread"))]
#[cfg_attr(not(feature = "enable_mt"), tokio::main(flavor = "current_thread"))]
async fn main() -> Result<(), AteError> {
    let opts: Opts = Opts::parse();
    //let opts = main_debug();

    let mut log_level = match opts.verbose {
        0 => "error",
        1 => "warn",
        2 => "info",
        _ => "debug",
    };
    if opts.debug { log_level = "debug"; }
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    let wire_encryption = match opts.wire_encryption {
        Some(a) => Some(a),
        None => {
            match opts.trust {
                TrustMode::Centralized => Some(KeySize::Bit128),
                TrustMode::Distributed => None
            }
        }
    };

    let mut conf = AteConfig::default();
    conf.dns_sec = opts.dns_sec;
    conf.dns_server = opts.dns_server;

    let auth = match opts.no_auth {
        false if opts.trust == TrustMode::Centralized => Some(opts.auth),
        _ => None,
    };
    
    match opts.subcmd {
        SubCommand::Solo(solo) => {
            main_solo(solo, conf, auth, opts.trust, wire_encryption).await?;
        }
    }

    info!("atedb::shutdown");

    Ok(())
}

async fn main_solo(solo: Solo, mut cfg_ate: ConfAte, auth: Option<url::Url>, trust: TrustMode, wire_encryption: Option<KeySize>) -> Result<(), AteError>
{
    // Create the chain flow and generate configuration
    cfg_ate.log_path = Some(shellexpand::tilde(&solo.logs_path).to_string());
    cfg_ate.compact_mode = solo.compact_mode
        .with_growth_factor(solo.compact_threshold_factor)
        .with_growth_size(solo.compact_threshold_size)
        .with_timer_value(Duration::from_secs(solo.compact_timer));
    
    // Create the chain flow and generate configuration
    let flow = ChainFlow::new(&cfg_ate, auth, solo.url.clone(), trust).await;

    // Create the server and listen on the port
    let mut cfg_mesh = ConfMesh::solo_from_url(&solo.url, &solo.listen)?;
    cfg_mesh.wire_protocol = StreamProtocol::parse(&solo.url)?;
    cfg_mesh.wire_encryption = wire_encryption;

    let server = create_server(&cfg_mesh).await?;
    server.add_route(Box::new(flow), &cfg_ate).await?;

    // Wait for ctrl-c
    eprintln!("Press ctrl-c to exit");
    let mut exit = ctrl_channel();
    while *exit.borrow() == false {
        exit.changed().await.unwrap();
    }
    println!("Goodbye!");
    Ok(())
}