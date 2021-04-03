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
use std::fs::File;

struct ChainFlow {
    root_key: PrivateSignKey,
    regex_auth: Regex,
    regex_cmd: Regex,
}

impl ChainFlow
{
    fn new(root_key: PrivateSignKey) -> Self {        
        ChainFlow {
            root_key,
            regex_auth: Regex::new("^/auth-[a-f0-9]{4}$").unwrap(),
            regex_cmd: Regex::new("^/cmd-[a-f0-9]{32}$").unwrap(),
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
                .add_root_public_key(&self.root_key.as_public_key())
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
        Ok(OpenAction::Deny(format!("The chain-key ({}) does not match a valid chain supported by this server.", key.to_string()).to_string()))
    }
}

#[derive(Clap)]
#[clap(version = "0.1", author = "John S. <johnathan.sharratt@gmail.com>")]
struct Opts {
    /// Sets the level of log verbosity, can be used multiple times
    #[clap(short, long, parse(from_occurrences))]
    verbose: i32,
    /// Logs debug info to the console
    #[clap(short, long)]
    debug: bool,
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    #[clap()]
    Run(Run),
    #[clap()]
    Generate(Generate),
}

/// Runs the login server
#[derive(Clap)]
struct Run {
    /// Path to the log files where all the authentication data is stored
    #[clap(index = 1, default_value = "~/ate/auth")]
    logs_path: String,
    /// Path to the secret key that helps protect key operations like creating users and resetting passwords
    #[clap(index = 2, default_value = "~/ate/auth.key")]
    key_path: String,
    /// IP address that the authentication server will isten on
    #[clap(short, long, default_value = "0.0.0.0")]
    listen: String,
    /// Port that the authentication server will listen on
    #[clap(short, long, default_value = "5000")]
    port: u16,
}

/// Generates the secret key that helps protect key operations like creating users and resetting passwords
#[derive(Clap)]
struct Generate {
    /// Path to the secret key
    #[clap(index = 1, default_value = "~/ate/auth.key")]
    key_path: String,
    /// Strength of the key that will be generated
    #[clap(short, long, default_value = "256")]
    strength: KeySize,
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

    // Prepare the logging
    let mut log_level = match opts.verbose {
        0 => "error",
        1 => "warn",
        2 => "info",
        _ => "debug",
    };
    if opts.debug { log_level = "debug"; }
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    // Determine what we need to do
    match opts.subcmd {
        SubCommand::Run(run) =>
        {
            // Open the key file
            let root_key = {
                let path = shellexpand::tilde(&run.key_path).to_string();
                let path = std::path::Path::new(&path);
                let _ = std::fs::create_dir_all(path.parent().unwrap().clone());
                let file = File::open(path).unwrap();
                bincode::deserialize_from(&file).unwrap()
            };

            // Create the chain flow and generate configuration
            let flow = ChainFlow::new(root_key);
            let mut cfg_ate = ate_auth::conf_auth();
            cfg_ate.log_path = run.logs_path;
            
            // Create the server and listen on port 5000
            let cfg_mesh = ConfMesh::solo(run.listen.as_str(), run.port);
            let _server = create_server(&cfg_ate, &cfg_mesh, Box::new(flow)).await;
            
            // Wait for ctrl-c
            let mut exit = ctrl_channel();
            while *exit.borrow() == false {
                exit.changed().await.unwrap();
            }
            println!("Goodbye!");
        },

        SubCommand::Generate(generate) => {
            let key = PrivateSignKey::generate(generate.strength);
            let path = shellexpand::tilde(&generate.key_path).to_string();
            let path = std::path::Path::new(&path);
            let _ = std::fs::create_dir_all(path.parent().unwrap().clone());
            let mut file = File::create(path).unwrap();
            
            print!("Generating secret key at {}...", generate.key_path);
            bincode::serialize_into(&mut file, &key).unwrap();
            println!("Done");
        },
    }

    // We are done
    Ok(())
}