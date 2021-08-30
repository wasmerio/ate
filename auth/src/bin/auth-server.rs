#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use ate::{prelude::*};
use ate_auth::prelude::*;
use clap::Clap;

#[derive(Clap)]
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

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), AteError>
{
    let opts: Opts = Opts::parse();

    ate::log_init(opts.verbose, opts.debug);

    // Determine what we need to do
    match opts.subcmd {
        SubCommand::Run(run) =>
        {
            // Open the key file
            let root_write_key: PrivateSignKey = ate_auth::load_key(run.key_path.clone(), ".write");
            let root_read_key: EncryptKey = ate_auth::load_key(run.key_path.clone(), ".read");
            let root_cert_key: PrivateEncryptKey = ate_auth::load_key(run.key_path.clone(), ".cert");
            
            // Build a session for service
            let mut cfg_ate = ate_auth::conf_auth();
            cfg_ate.log_path = Some(shellexpand::tilde(&run.logs_path).to_string());
            if let Some(backup_path) = run.backup_path {
                cfg_ate.backup_path = Some(shellexpand::tilde(&backup_path).to_string());
            }
            cfg_ate.compact_mode = CompactMode::Never;
            
            let mut session = AteSessionUser::new();
            session.user.add_read_key(&root_read_key);
            session.user.add_write_key(&root_write_key);

            // Create the server and listen
            let mut flow = ChainFlow::new(&cfg_ate, root_write_key, session, &run.url);
            flow.terms_and_conditions = Some(ate_auth::GENERIC_TERMS_AND_CONDITIONS.to_string());
            let mut cfg_mesh = ConfMesh::solo_from_url(&cfg_ate, &run.url, &run.listen, None, run.node_id).await?;
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
        },

        SubCommand::Generate(generate) => {
            let read_key = EncryptKey::generate(generate.strength);
            ate_auth::save_key(generate.key_path.clone(), read_key, ".read");

            let write_key = PrivateSignKey::generate(generate.strength);
            ate_auth::save_key(generate.key_path.clone(), write_key, ".write");

            let cert_key = PrivateEncryptKey::generate(generate.strength);
            ate_auth::save_key(generate.key_path.clone(), cert_key, ".cert");
        },
    }

    // We are done
    Ok(())
}