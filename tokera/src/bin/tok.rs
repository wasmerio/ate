#![allow(unused_imports)]
use ate::prelude::*;
use clap::Parser;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
#[cfg(feature = "bus")]
use tokera::bus::*;
use tokera::cmd::*;
use tokera::error::*;
use tokera::opt::*;
use tokera::prelude::*;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

#[allow(dead_code)]
#[derive(Parser)]
#[clap(version = "1.2", author = "Tokera Pty Ltd <info@tokera.com>")]
struct Opts {
    /// Sets the level of log verbosity, can be used multiple times
    #[clap(short, long, parse(from_occurrences))]
    pub verbose: i32,
    /// Token file to read that holds a previously created token to be used for this operation
    #[cfg(not(target_os = "wasi"))]
    #[clap(long, default_value = "~/tok/token")]
    pub token_path: String,
    /// Token file to read that holds a previously created token to be used for this operation
    #[cfg(target_os = "wasi")]
    #[clap(long, default_value = "/.private/token")]
    pub token_path: String,
    /// URL that this command will send all its authentication requests to (e.g. wss://tokera.sh/auth)
    #[clap(long)]
    pub auth_url: Option<url::Url>,
    /// NTP server address that the file-system will synchronize with
    #[clap(long)]
    pub ntp_pool: Option<String>,
    /// NTP server port that the file-system will synchronize with
    #[clap(long)]
    pub ntp_port: Option<u16>,
    /// Determines if ATE will use DNSSec or just plain DNS
    #[clap(long)]
    pub dns_sec: bool,
    /// Address that DNS queries will be sent to
    #[clap(long, default_value = "8.8.8.8")]
    pub dns_server: String,
    /// Logs debug info to the console
    #[clap(short, long)]
    pub debug: bool,
    #[clap(subcommand)]
    pub subcmd: SubCommand,
}

#[derive(Parser)]
enum SubCommand {
    /// Users are personal accounts and services that have an authentication context.
    /// Every user comes with a personal wallet that can hold commodities.
    #[clap()]
    User(OptsUser),
    /// Domain groups are collections of users that share something together in association
    /// with an internet domain name. Every group has a built in wallet(s) that you can
    /// use instead of a personal wallet. In order to claim a domain group you will need
    /// DNS access to an owned internet domain that can be validated.
    #[clap()]
    Domain(OptsDomain),
    /// Databases are chains of data that make up a particular shard. These databases can be
    /// use for application data persistance, file systems and web sites.
    #[clap()]
    Db(OptsDatabase),
    /// Starts the process in BUS mode which will allow it to accept calls from other
    /// processes.
    #[cfg(feature = "bus")]
    #[clap()]
    Bus(OptsBus),
    /// Tokens are stored authentication and authorization secrets used by other processes.
    /// Using this command you may generate a custom token however the usual method for
    /// authentication is to use the login command instead.
    #[cfg(not(feature_os = "wasi"))]
    #[clap()]
    Token(OptsToken),
    /// Services offered by Tokera (and other 3rd parties) are accessible via this
    /// sub command menu, including viewing the available services and subscribing
    /// to them.
    #[clap()]
    Service(OptsService),
    /// Contracts represent all the subscriptions you have made to specific services
    /// you personally consume or a group consume that you act on your authority on
    /// behalf of. This sub-menu allows you to perform actions such as cancel said
    /// contracts.
    #[clap()]
    Contract(OptsContract),
    /// Instances are running web assembly applications that can accessed from
    /// anywhere via API calls and/or the wasm-bus.
    #[clap()]
    Instance(OptsInstance),
    /// Wallets are directly attached to groups and users - they hold a balance,
    /// store transaction history and facilitate transfers, deposits and withdraws.
    #[clap()]
    Wallet(OptsWallet),
    /// Login to an account and store the token locally for reuse.
    #[clap()]
    Login(OptsLogin),
    /// Logout of the account by deleting the local token.
    #[clap()]
    Logout(OptsLogout),
}

#[allow(dead_code)]
fn binary_path(args: &mut impl Iterator<Item = OsString>) -> PathBuf {
    match args.next() {
        Some(ref s) if !s.is_empty() => PathBuf::from(s),
        _ => std::env::current_exe().unwrap(),
    }
}

#[allow(dead_code)]
fn name(binary_path: &Path) -> &str {
    binary_path.file_stem().unwrap().to_str().unwrap()
}

#[cfg(target_os = "wasi")]
async fn init_wasi_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        if let Some(location) = panic_info.location() {
            println!(
                "panic occurred in file '{}' at line {}",
                location.file(),
                location.line(),
            );
        } else {
            println!("panic occurred but can't get location information...");
        }
        eprintln!("{:?}", backtrace::Backtrace::new());
    }));
}

#[cfg(target_os = "wasi")]
async fn init_wasi_ws() {
    // Add the main Tokera certificate and connect via a wasm_bus web socket
    add_global_certificate(&AteHash::from_hex_string("9c960f3ba2ece59881be0b45f39ef989").unwrap());
    set_comm_factory(move |addr| {
        let schema = match addr.port() {
            80 => "ws",
            _ => "wss",
        };
        let addr = url::Url::from_str(format!("{}://{}", schema, addr).as_str()).unwrap();
        Box::pin(async move {
            tracing::trace!("opening wasm_bus::web_socket");
            let ws = wasm_bus_ws::prelude::SocketBuilder::new(addr)
                .open()
                .await
                .unwrap();
            Some(ate::comms::Stream::WasmWebSocket(ws))
        })
    })
    .await;
}

#[cfg(target_arch = "wasm32")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    wasm_bus::task::block_on(main_async())
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    main_async().await?;
    std::process::exit(0);
}

async fn main_async() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logging and panic hook
    #[cfg(target_os = "wasi")]
    init_wasi_hook().await;

    // Set the origin
    #[cfg(target_os = "wasi")]
    init_wasi_ws().await;

    // Allow for symbolic links to the main binary
    let args = wild::args_os().collect::<Vec<_>>();
    let mut args = args.iter().cloned();
    let binary = binary_path(&mut args);
    let binary_as_util = name(&binary);

    let mut opts = {
        let cmd = match binary_as_util {
            "user" => Some(SubCommand::User(OptsUser::parse())),
            "domain" => Some(SubCommand::Domain(OptsDomain::parse())),
            "db" => Some(SubCommand::Db(OptsDatabase::parse())),
            #[cfg(feature = "bus")]
            "bus" => Some(SubCommand::Bus(OptsBus::parse())),
            "service" => Some(SubCommand::Service(OptsService::parse())),
            "inst" => Some(SubCommand::Instance(OptsInstance::parse())),
            "instance" => Some(SubCommand::Instance(OptsInstance::parse())),
            "contract" => Some(SubCommand::Contract(OptsContract::parse())),
            "wallet" => Some(SubCommand::Wallet(OptsWallet::parse())),
            "login" => Some(SubCommand::Login(OptsLogin::parse())),
            "logout" => Some(SubCommand::Logout(OptsLogout::parse())),
            _ => None,
        };
        match cmd {
            Some(cmd) => Opts {
                verbose: 1,
                #[cfg(not(target_os = "wasi"))]
                token_path: "~/tok/token".to_string(),
                #[cfg(target_os = "wasi")]
                token_path: "/.private/token".to_string(),
                auth_url: None,
                ntp_pool: None,
                ntp_port: None,
                dns_sec: false,
                dns_server: "8.8.8.8".to_string(),
                debug: false,
                subcmd: cmd,
            },
            None => Opts::parse(),
        }
    };

    // We upgrade the verbosity for certain commands by default
    opts.verbose = opts.verbose.max(match &opts.subcmd {
        #[cfg(feature = "bus")]
        SubCommand::Bus(..) => 4,
        _ => 0,
    });

    ate::log_init(opts.verbose, opts.debug);
    let auth = ate_auth::prelude::origin_url(&opts.auth_url, "auth");

    // Build the ATE configuration object
    let mut conf = AteConfig::default();
    conf.dns_sec = opts.dns_sec;
    conf.dns_server = opts.dns_server;
    #[cfg(feature = "enable_ntp")]
    if let Some(pool) = opts.ntp_pool {
        conf.ntp_pool = pool;
    }
    #[cfg(feature = "enable_ntp")]
    if let Some(port) = opts.ntp_port {
        conf.ntp_port = port;
    }

    // If the domain is localhost then load certificates from dev.tokera.com
    #[cfg(feature = "enable_dns")]
    if let Some(domain) = auth.domain() {
        if domain == "localhost" {
            let test_registry = Registry::new(&conf).await;
            for cert in test_registry.dns_certs("dev.tokera.com").await.unwrap() {
                add_global_certificate(&cert);
            }
        }
    }

    // Do we need a token
    let needs_token = match &opts.subcmd {
        SubCommand::Login(..) => false,
        SubCommand::Token(..) => false,
        #[cfg(feature = "bus")]
        SubCommand::Bus(..) => false,
        SubCommand::User(a) => match a.action {
            UserAction::Create(..) => false,
            UserAction::Recover(..) => false,
            _ => true,
        },
        _ => true,
    };

    // Make sure the token exists
    if needs_token {
        let token_path = shellexpand::tilde(&opts.token_path).to_string();
        if std::path::Path::new(&token_path).exists() == false {
            eprintln!("Token not found - please first login.");
            std::process::exit(1);
        }
    }

    // Determine what we need to do
    match opts.subcmd {
        SubCommand::User(opts_user) => {
            main_opts_user(opts_user, None, Some(opts.token_path), auth).await?;
        }
        SubCommand::Domain(opts_group) => {
            main_opts_group(opts_group, None, Some(opts.token_path), auth, "Domain name").await?;
        }
        SubCommand::Db(opts_db) => {
            main_opts_db(opts_db, None, Some(opts.token_path), auth, "Domain name").await?;
        }
        #[cfg(feature = "bus")]
        SubCommand::Bus(opts_bus) => {
            main_opts_bus(opts_bus, conf, opts.token_path, auth).await?;
        }
        #[cfg(not(feature_os = "wasi"))]
        SubCommand::Token(opts_token) => {
            main_opts_token(opts_token, None, Some(opts.token_path), auth, "Domain name").await?;
        }
        SubCommand::Wallet(opts_wallet) => {
            main_opts_wallet(opts_wallet.source, opts.token_path, auth).await?
        }
        SubCommand::Contract(opts_contract) => {
            main_opts_contract(opts_contract.purpose, opts.token_path, auth).await?;
        }
        SubCommand::Service(opts_service) => {
            main_opts_service(opts_service.purpose, opts.token_path, auth).await?;
        }
        SubCommand::Instance(opts_instance) => {
            let db_url = ate_auth::prelude::origin_url(&opts_instance.db_url, "db");
            let sess_url = ate_auth::prelude::origin_url(&opts_instance.sess_url, "sess");
            main_opts_instance(opts_instance.purpose, opts.token_path, auth, db_url, sess_url, opts_instance.ignore_certificate).await?;
        }
        SubCommand::Login(opts_login) => main_opts_login(opts_login, opts.token_path, auth).await?,
        SubCommand::Logout(opts_logout) => main_opts_logout(opts_logout, opts.token_path).await?,
    }

    // We are done
    Ok(())
}
