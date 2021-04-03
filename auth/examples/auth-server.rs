#![allow(unused_imports)]
use async_trait::async_trait;
use log::{info, warn, debug, error};
use serde::{Serialize, Deserialize};
use regex::Regex;
use ate::{error::ChainCreationError, prelude::*};
use ate_auth::*;
use clap::Clap;
use std::time::Duration;
use ctrlc::*;
use tokio::sync::watch;

struct ChainFlow {
    regex_auth: Regex,
    regex_cmd: Regex,
}

impl Default
for ChainFlow
{
    fn default() -> Self {        
        ChainFlow {
            regex_auth: Regex::new(r"^auth-[a-f0-9]{4}$").unwrap(),
            regex_cmd: Regex::new(r"^cmd-[a-f0-9]{32}$").unwrap(),
        }
    }
}

#[async_trait]
impl OpenFlow
for ChainFlow
{
    async fn open(&self, cfg: &ConfAte, key: &ChainKey) -> Result<OpenAction, ChainCreationError>
    {
        let name = key.name.clone();
        let name = name.as_str();
        if self.regex_auth.is_match(name) {
            let chain = ChainBuilder::new(cfg)
                .build(key)
                .await?;
            return Ok(OpenAction::Chain(chain));
        }
        if self.regex_cmd.is_match(name) {
            let chain = ChainBuilder::new(cfg)
                .temporal(true)
                .build(key)
                .await?;
            return Ok(OpenAction::Chain(chain));
        }
        Ok(OpenAction::Deny("The chain-key does not match a valid chain supported by this server.".to_string()))
    }
}

#[derive(Clap)]
#[clap(version = "0.1", author = "John S. <johnathan.sharratt@gmail.com>")]
struct Opts {
    /// Sets the level of log verbosity, can be used multiple times
    #[clap(short, long, parse(from_occurrences))]
    verbose: i32,
    /// IP address that the authentication server will isten on
    #[clap(short, long, default_value = "0.0.0.0")]
    listen: String,
    /// Port that the authentication server will listen on
    #[clap(short, long, default_value = "5000")]
    port: u16,
    /// Logs debug info to the console
    #[clap(short, long)]
    debug: bool,
}

fn ctrl_channel() -> tokio::sync::watch::Receiver<bool> {
    let (sender, receiver) = tokio::sync::watch::channel(false);
    ctrlc::set_handler(move || {
        let _ = sender.send(true);
    }).unwrap();
    receiver
}

#[tokio::main]
async fn main() -> Result<(), AteError>
{
    let opts: Opts = Opts::parse();

    let mut log_level = match opts.verbose {
        1 => "info",
        2 => "debug",
        _ => "error",
    };
    if opts.debug { log_level = "debug"; }
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    // Create the server and listen on port 5000
    let cfg_mesh = ConfMesh::solo(opts.listen.as_str(), opts.port);
    let cfg_ate = ConfAte::default();
    let _server = create_persistent_server(&cfg_ate, &cfg_mesh).await;
    
    // Wait for ctrl-c
    let mut exit = ctrl_channel();
    while *exit.borrow() == false {
        exit.changed().await.unwrap();
    }
    println!("Goodbye!");

    Ok(())
}