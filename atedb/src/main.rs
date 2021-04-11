#![allow(unused_imports, dead_code)]
use log::{info, error, debug};
use ate::prelude::*;
use std::env;
use std::io::ErrorKind;
use directories::BaseDirs;
use std::sync::Arc;
use std::ops::Deref;
use url::Url;
use tokio::select;

use clap::Clap;

mod flow;

use crate::flow::ChainFlow;

#[derive(Clap)]
#[clap(version = "0.1", author = "John S. <johnathan.sharratt@gmail.com>")]
struct Opts {
    /// Sets the level of log verbosity, can be used multiple times
    #[allow(dead_code)]
    #[clap(short, long, parse(from_occurrences))]
    verbose: i32,
    /// Logs debug info to the console
    #[clap(short, long)]
    debug: bool,
    /// URL where the user is authenticated
    #[clap(short, long, default_value = "tcp://auth.tokera.com:5001/auth")]
    auth: Url,
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
    /// IP address that the database server will isten on
    #[clap(short, long, default_value = "0.0.0.0")]
    listen: String,
    /// Port that the database server will listen on
    #[clap(short, long, default_value = "5000")]
    port: u16,
}

fn ctrl_channel() -> tokio::sync::watch::Receiver<bool> {
    let (sender, receiver) = tokio::sync::watch::channel(false);
    ctrlc::set_handler(move || {
        let _ = sender.send(true);
    }).unwrap();
    receiver
}

#[tokio::main]
async fn main() -> Result<(), AteError> {
    //let opts: Opts = Opts::parse();
    let opts = main_debug();

    let mut log_level = match opts.verbose {
        0 => "error",
        1 => "warn",
        2 => "info",
        _ => "debug",
    };
    if opts.debug { log_level = "debug"; }
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    let mut conf = AteConfig::default();
    conf.dns_sec = opts.dns_sec;
    conf.dns_server = opts.dns_server;
    
    match opts.subcmd {
        SubCommand::Solo(solo) => {
            main_solo(solo, conf, opts.auth).await?;
        }
    }

    info!("atefs::shutdown");

    Ok(())
}

async fn main_solo(solo: Solo, mut cfg_ate: ConfAte, auth: url::Url) -> Result<(), AteError>
{
    // Create the chain flow and generate configuration
    cfg_ate.log_path = shellexpand::tilde(&solo.logs_path).to_string();

    // Create the chain flow and generate configuration
    let flow = ChainFlow::new(&cfg_ate, auth);

    // Create the server and listen on port 5000
    let cfg_mesh = ConfMesh::solo(solo.listen.as_str(), solo.port);
    let _server = create_server(&cfg_ate, &cfg_mesh, Box::new(flow)).await;

    // Wait for ctrl-c
    eprintln!("Press ctrl-c to exit");
    let mut exit = ctrl_channel();
    while *exit.borrow() == false {
        exit.changed().await.unwrap();
    }
    println!("Goodbye!");
    Ok(())
}