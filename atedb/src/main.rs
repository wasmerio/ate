use ate::{compact::CompactMode, prelude::*, utils::load_node_list};
use std::time::Duration;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use url::Url;

use clap::Parser;

mod flow;

use crate::flow::ChainFlow;

#[derive(Parser)]
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
    #[clap(short, long, default_value = "ws://wasmer.sh/auth")]
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
    /// Trust mode that the datachain server will run under - valid values are either
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

#[derive(Parser)]
enum SubCommand {
    #[clap()]
    Solo(Solo),
}
/// Runs a solo ATE datachain and listens for connections from clients
#[derive(Parser)]
struct Solo {
    /// Path to the log files where all the file system data is stored
    #[clap(index = 1, default_value = "/opt/ate")]
    logs_path: String,
    /// Path to the backup and restore location of log files
    #[clap(short, long)]
    backup_path: Option<String>,
    /// Address that the datachain server(s) are listening and that
    /// this server can connect to if the chain is on another mesh node
    #[clap(short, long, default_value = "ws://localhost:5000/db")]
    url: url::Url,
    /// Optional list of the nodes that make up this cluster
    /// (if the file does not exist then it will not load)
    #[clap(long)]
    nodes_list: Option<String>,
    /// IP address that the datachain server will isten on
    #[clap(short, long, default_value = "::")]
    listen: IpAddr,
    /// Ensures that this datachain runs as a specific node_id
    #[clap(short, long)]
    node_id: Option<u32>,
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
    ctrlc_async::set_handler(move || {
        let _ = sender.send(true);
    })
    .unwrap();
    receiver
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), AteError> {
    let opts: Opts = Opts::parse();
    //let opts = main_debug();

    ate::log_init(opts.verbose, opts.debug);

    let wire_encryption = match opts.wire_encryption {
        Some(a) => Some(a),
        None => match opts.trust.is_centralized() {
            true => match opts.no_wire_encryption {
                false => Some(KeySize::Bit128),
                true => None,
            },
            false => None,
        },
    };

    let mut conf = AteConfig::default();
    conf.dns_sec = opts.dns_sec;
    conf.dns_server = opts.dns_server;

    let auth = match opts.no_auth {
        false if opts.trust.is_centralized() => Some(opts.auth),
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

async fn main_solo(
    solo: Solo,
    mut cfg_ate: ConfAte,
    auth: Option<url::Url>,
    trust: TrustMode,
    wire_encryption: Option<KeySize>,
) -> Result<(), AteError> {
    // Create the chain flow and generate configuration
    cfg_ate.log_path = Some(shellexpand::tilde(&solo.logs_path).to_string());
    cfg_ate.backup_path = solo
        .backup_path
        .as_ref()
        .map(|a| shellexpand::tilde(a).to_string());
    cfg_ate.compact_mode = solo
        .compact_mode
        .with_growth_factor(solo.compact_threshold_factor)
        .with_growth_size(solo.compact_threshold_size)
        .with_timer_value(Duration::from_secs(solo.compact_timer));
    cfg_ate.nodes = load_node_list(solo.nodes_list);

    // Create the chain flow and generate configuration
    let flow = ChainFlow::new(&cfg_ate, auth, solo.url.clone(), trust).await;

    // Create the server and listen on the port
    let mut cfg_mesh =
        ConfMesh::solo_from_url(&cfg_ate, &solo.url, &solo.listen, None, solo.node_id).await?;
    cfg_mesh.wire_protocol = StreamProtocol::parse(&solo.url)?;
    cfg_mesh.wire_encryption = wire_encryption;

    let server = create_server(&cfg_mesh).await?;
    server.add_route(Box::new(flow), &cfg_ate).await?;

    // Wait for ctrl-c
    println!("Press ctrl-c to exit");
    let mut exit = ctrl_channel();
    while *exit.borrow() == false {
        exit.changed().await.unwrap();
    }
    println!("Shutting down...");
    server.shutdown().await;
    println!("Goodbye!");
    Ok(())
}
