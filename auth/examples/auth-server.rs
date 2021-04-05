#[allow(unused_imports)]
use log::{info, warn, debug, error};
use ate::{prelude::*};
use ate_auth::prelude::*;
use clap::Clap;
use std::fs::File;

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

            
            // Build a session for service
            let mut cfg_ate = ate_auth::conf_auth();
            cfg_ate.log_path = shellexpand::tilde(&run.logs_path).to_string();
            let session = AteSession::new(&cfg_ate);

            // Create the chain flow and generate configuration
            let flow = ChainFlow::new(root_key, session);

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