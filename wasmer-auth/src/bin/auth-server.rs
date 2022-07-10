use ate::prelude::*;
use clap::Parser;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use wasmer_auth::helper::*;
use wasmer_auth::prelude::*;

#[derive(Parser)]
#[clap(version = "1.5", author = "John S. <johnathan.sharratt@gmail.com>")]
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

#[derive(Parser)]
enum SubCommand {
    #[clap()]
    Run(Run),
    #[clap()]
    Generate(Generate),
}

/// Runs the login authentication and authorization server
#[derive(Parser)]
struct Run {
    /// Path to the secret key that helps protect key operations like creating users and resetting passwords
    #[clap(long, default_value = "~/wasmer/auth.key")]
    auth_key_path: String,
    /// Path to the secret key that grants access to the WebServer role within groups
    #[clap(long, default_value = "~/wasmer/web.key")]
    web_key_path: String,
    /// Path to the secret key that grants access to the EdgeCompute role within groups
    #[clap(long, default_value = "~/wasmer/edge.key")]
    edge_key_path: String,
    /// Path to the secret key that grants access to the contracts
    #[clap(long, default_value = "~/wasmer/contract.key")]
    contract_key_path: String,
    /// Path to the log files where all the authentication data is stored
    #[clap(index = 1, default_value = "~/wasmer/auth")]
    logs_path: String,
    /// Path to the backup and restore location of log files
    #[clap(short, long)]
    backup_path: Option<String>,
    /// Address that the authentication server(s) are listening and that
    /// this server can connect to if the chain is on another mesh node
    #[clap(short, long, default_value = "ws://localhost:5001/auth")]
    url: url::Url,
    /// IP address that the authentication server will isten on
    #[clap(short, long, default_value = "::")]
    listen: IpAddr,
    /// Ensures that this authentication server runs as a specific node_id
    #[clap(short, long)]
    node_id: Option<u32>,
}

/// Generates the secret key that helps protect key operations like creating users and resetting passwords
#[derive(Parser)]
struct Generate {
    /// Path to the secret key
    #[clap(index = 1, default_value = "~/wasmer/")]
    key_path: String,
    /// Strength of the key that will be generated
    #[clap(short, long, default_value = "256")]
    strength: KeySize,
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

    ate::log_init(opts.verbose, opts.debug);

    // Determine what we need to do
    match opts.subcmd {
        SubCommand::Run(run) => {
            // Open the key file
            let root_write_key: PrivateSignKey = load_key(run.auth_key_path.clone(), ".write");
            let root_read_key: EncryptKey = load_key(run.auth_key_path.clone(), ".read");
            let root_cert_key: PrivateEncryptKey = load_key(run.auth_key_path.clone(), ".cert");
            let web_key: EncryptKey = load_key(run.web_key_path.clone(), ".read");
            let edge_key: EncryptKey = load_key(run.edge_key_path.clone(), ".read");
            let contract_key: EncryptKey = load_key(run.contract_key_path.clone(), ".read");

            // Build a session for service
            let mut cfg_ate = conf_auth();
            cfg_ate.log_path = Some(shellexpand::tilde(&run.logs_path).to_string());
            if let Some(backup_path) = run.backup_path {
                cfg_ate.backup_path = Some(shellexpand::tilde(&backup_path).to_string());
            }
            cfg_ate.compact_mode = CompactMode::Never;

            let mut session = AteSessionUser::new();
            session.user.add_read_key(&root_read_key);
            session.user.add_write_key(&root_write_key);

            // Create the server and listen
            let mut flow = ChainFlow::new(
                &cfg_ate,
                root_write_key,
                session,
                web_key,
                edge_key,
                contract_key,
                &run.url,
            );
            flow.terms_and_conditions = Some(wasmer_auth::GENERIC_TERMS_AND_CONDITIONS.to_string());
            let mut cfg_mesh =
                ConfMesh::solo_from_url(&cfg_ate, &run.url, &run.listen, None, run.node_id).await?;
            cfg_mesh.wire_protocol = StreamProtocol::parse(&run.url)?;
            cfg_mesh.listen_certificate = Some(root_cert_key);

            let server = create_server(&cfg_mesh).await?;
            server.add_route(Box::new(flow), &cfg_ate).await?;

            // Wait for ctrl-c
            let mut exit = ctrl_channel();
            while *exit.borrow() == false {
                exit.changed().await.unwrap();
            }
            println!("Shutting down...");
            server.shutdown().await;
            println!("Goodbye!");
        }

        SubCommand::Generate(generate) => {
            let mut key_path = generate.key_path.clone();
            if key_path.ends_with("/") == false {
                key_path += "/";
            }

            let read_key = EncryptKey::generate(generate.strength);
            save_key(key_path.clone(), read_key, "auth.key.read");

            let write_key = PrivateSignKey::generate(generate.strength);
            save_key(key_path.clone(), write_key, "auth.key.write");

            let cert_key = PrivateEncryptKey::generate(generate.strength);
            save_key(key_path.clone(), cert_key, "auth.key.cert");

            let web_key = EncryptKey::generate(generate.strength);
            save_key(key_path.clone(), web_key, "web.key.read");

            let edge_key = EncryptKey::generate(generate.strength);
            save_key(key_path.clone(), edge_key, "edge.key.read");

            let contract_key = EncryptKey::generate(generate.strength);
            save_key(key_path.clone(), contract_key, "contract.key.read");
        }
    }

    // We are done
    Ok(())
}
