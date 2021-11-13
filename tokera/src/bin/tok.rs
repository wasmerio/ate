#![allow(unused_imports)]
use tracing::{info, warn, debug, error};
use ate::{prelude::*};
use clap::Parser;
use tokera::prelude::*;
use tokera::error::*;
use tokera::opt::*;
use tokera::cmd::*;

#[allow(dead_code)]
#[derive(Parser)]
#[clap(version = "1.5", author = "Tokera Pty Ltd <info@tokera.com>")]
struct Opts {
    /// Sets the level of log verbosity, can be used multiple times
    #[clap(short, long, parse(from_occurrences))]
    pub verbose: i32,
    /// Token file to read that holds a previously created token to be used for this operation
    #[clap(long, default_value = "~/tok/token")]
    pub token_path: String,
    /// URL that this command will send all its authentication requests to
    #[clap(long, default_value = "ws://tokera.com/auth")]
    pub auth_url: String,
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
    /// Tokens are stored authentication and authorization secrets used by other processes.
    /// Using this command you may generate a custom token however the usual method for
    /// authentication is to use the login command instead.
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

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>>
{
    let opts: Opts = Opts::parse();
    //let opts = debug_opts();
    ate::log_init(opts.verbose, opts.debug);

    // Load the authentication address
    let auth = url::Url::parse(opts.auth_url.as_str())?;

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
    #[cfg(feature="enable_dns")]
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
        SubCommand::Login( .. ) => false,
        SubCommand::Token( .. ) => false,
        _ => true
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
        },
        SubCommand::Domain(opts_group) => {
            main_opts_group(opts_group, None, Some(opts.token_path), auth, "Domain name").await?;
        },
        SubCommand::Token(opts_token) => {
            main_opts_token(opts_token, None, Some(opts.token_path), auth, "Domain name").await?;
        }
        SubCommand::Wallet(opts_wallet) => {
            main_opts_wallet(opts_wallet.source, opts.token_path, auth).await?
        },
        SubCommand::Contract(opts_contract) => {
            main_opts_contract(opts_contract.purpose, opts.token_path, auth).await?;
        },
        SubCommand::Service(opts_service) => {
            main_opts_service(opts_service.purpose, opts.token_path, auth).await?;
        },
        SubCommand::Login(opts_login) => {
            main_opts_login(opts_login, opts.token_path, auth).await?
        },
        SubCommand::Logout(opts_logout) => {
            main_opts_logout(opts_logout, opts.token_path).await?
        }
    }

    // We are done
    Ok(())
}