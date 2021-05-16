#[allow(unused_imports)]
use log::{info, warn, debug, error};
use serde::*;
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

/// Runs the login authentication and authorization server
#[derive(Clap)]
struct Run {
    /// Path to the secret key that helps protect key operations like creating users and resetting passwords
    #[clap(index = 1, default_value = "~/ate/auth.key")]
    key_path: String,
    /// Path to the log files where all the authentication data is stored
    #[clap(index = 2, default_value = "~/ate/auth")]
    logs_path: String,
    /// IP address that the authentication server will isten on
    #[clap(short, long, default_value = "0.0.0.0")]
    listen: String,
    /// Port that the authentication server will listen on
    #[clap(short, long, default_value = "5001")]
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

fn load_key<T>(key_path: String, postfix: &str) -> T
where T: serde::de::DeserializeOwned
{
    let key_path = format!("{}{}", key_path, postfix).to_string();
    let path = shellexpand::tilde(&key_path).to_string();
    let path = std::path::Path::new(&path);
    let _ = std::fs::create_dir_all(path.parent().unwrap().clone());
    let file = File::open(path).unwrap();
    bincode::deserialize_from(&file).unwrap()
}

fn save_key<T>(key_path: String, key: T, postfix: &str)
where T: Serialize
{
    let key_path = format!("{}{}", key_path, postfix).to_string();
    let path = shellexpand::tilde(&key_path).to_string();
    let path = std::path::Path::new(&path);
    let _ = std::fs::create_dir_all(path.parent().unwrap().clone());
    let mut file = File::create(path).unwrap();
    
    print!("Generating secret key at {}...", key_path);
    bincode::serialize_into(&mut file, &key).unwrap();
    println!("Done");
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
            let root_write_key: PrivateSignKey = load_key(run.key_path.clone(), ".write");
            let root_read_key: EncryptKey = load_key(run.key_path.clone(), ".read");
            
            // Build a session for service
            let mut cfg_ate = ate_auth::conf_auth();
            cfg_ate.log_path = Some(shellexpand::tilde(&run.logs_path).to_string());
            cfg_ate.compact_mode = CompactMode::Never;
            let mut session = AteSession::new(&cfg_ate);
            session.user.add_read_key(&root_read_key);
            session.user.add_write_key(&root_write_key);

            // Create the chain flow and generate configuration
            let flow = ChainFlow::new(&cfg_ate, root_write_key, session);

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
            let read_key = EncryptKey::generate(generate.strength);
            save_key(generate.key_path.clone(), read_key, ".read");

            let write_key = PrivateSignKey::generate(generate.strength);
            save_key(generate.key_path.clone(), write_key, ".write");
        },
    }

    // We are done
    Ok(())
}