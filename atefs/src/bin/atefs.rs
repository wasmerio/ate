use ate::prelude::*;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use atefs::main_mount;
use atefs::opts::*;

use ate_auth::cmd::*;
use ate_auth::helper::*;

#[cfg(feature = "enable_tokera")]
use tokera::cmd::{
    main_opts_contract, main_opts_login, main_opts_logout, main_opts_service, main_opts_wallet,
};

use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();
    //let opts = test_opts();

    ate::log_init(opts.verbose, opts.debug);

    let mut conf = AteConfig::default();
    conf.dns_sec = opts.dns_sec;
    conf.dns_server = opts.dns_server;
    conf.ntp_sync = opts.no_ntp == false;

    if let Some(pool) = opts.ntp_pool {
        conf.ntp_pool = pool;
    }
    if let Some(port) = opts.ntp_port {
        conf.ntp_port = port;
    }

    #[cfg(feature = "enable_tokera")]
    let token_path = { Some(opts.token_path.clone()) };
    #[cfg(not(feature = "enable_tokera"))]
    let token_path = opts.token_path.clone();

    // Do we need a token
    let needs_token = match &opts.subcmd {
        SubCommand::Login(..) => false,
        SubCommand::Token(..) => false,
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

    match opts.subcmd {
        SubCommand::Token(opts_token) => {
            main_opts_token(opts_token, opts.token, token_path, opts.auth, "Domain name").await?;
        }
        SubCommand::User(opts_user) => {
            main_opts_user(opts_user, opts.token, token_path, opts.auth).await?;
        }
        SubCommand::Db(opts_db) => {
            main_opts_db(opts_db, opts.token, token_path, opts.auth, "Domain name").await?;
        }
        #[cfg(not(feature = "enable_tokera"))]
        SubCommand::Group(opts_group) => {
            if opts.no_auth {
                eprintln!("In order to create groups you must use some form of authentication.");
                std::process::exit(1);
            }
            main_opts_group(opts_group, opts.token, token_path, opts.auth, "Group").await?;
        }
        #[cfg(feature = "enable_tokera")]
        SubCommand::Domain(opts_group) => {
            main_opts_group(opts_group, None, token_path, opts.auth, "Domain name").await?;
        }
        #[cfg(feature = "enable_tokera")]
        SubCommand::Wallet(opts_wallet) => {
            main_opts_wallet(opts_wallet.source, opts.token_path, opts.auth).await?
        }
        #[cfg(feature = "enable_tokera")]
        SubCommand::Contract(opts_contract) => {
            main_opts_contract(opts_contract.purpose, opts.token_path, opts.auth).await?;
        }
        #[cfg(feature = "enable_tokera")]
        SubCommand::Service(opts_service) => {
            main_opts_service(opts_service.purpose, opts.token_path, opts.auth).await?;
        }
        #[cfg(feature = "enable_tokera")]
        SubCommand::Login(opts_login) => {
            main_opts_login(opts_login, opts.token_path, opts.auth).await?
        }
        #[cfg(feature = "enable_tokera")]
        SubCommand::Logout(opts_logout) => main_opts_logout(opts_logout, opts.token_path).await?,
        SubCommand::Mount(mount) => {
            // Derive the group from the mount address
            let mut group = None;
            if let Some(remote) = &mount.remote_name {
                if let Some((group_str, _)) = remote.split_once("/") {
                    group = Some(group_str.to_string());
                }
            }

            let mut session: AteSessionType = AteSessionUser::default().into();

            // If a passcode is supplied then use this
            if let Some(pass) = &mount.passcode {
                if opts.token.is_some() || token_path.is_some() {
                    eprintln!("You can not supply both a passcode and a token, either drop the --token arguments or the --passcode argument");
                    std::process::exit(1);
                }
                if mount.remote_name.is_some() {
                    eprintln!("Using a passcode is not compatible with remotely hosted file-systems as the distributed datchain needs to make authentication checks");
                    std::process::exit(1);
                }

                let prefix = "ate:".to_string();
                let key = password_to_read_key(&prefix, &pass, 15, KeySize::Bit192);

                let mut session_user = AteSessionUser::default();
                session_user.user.add_read_key(&key);
                session = session_user.into();
            } else if opts.no_auth {
                if mount.remote_name.is_some() {
                    eprintln!("In order to use remotely hosted file-systems you must use some form of authentication, without authentication the distributed databases will not be able to make the needed checks");
                    std::process::exit(1);
                }

                // We do not put anything in the session as no authentication method nor a passcode was supplied
            } else {
                // Load the session via the token or the authentication server
                let session_user = main_session_user(
                    opts.token.clone(),
                    token_path.clone(),
                    Some(opts.auth.clone()),
                )
                .await?;

                // Attempt to grab additional permissions for the group (if it has any)
                session = if group.is_some() {
                    match main_gather(
                        group.clone(),
                        session_user.clone().into(),
                        opts.auth,
                        "Group",
                    )
                    .await
                    {
                        Ok(a) => a.into(),
                        Err(err) => {
                            debug!("Group authentication failed: {} - falling back to user level authorization", err);
                            session_user.into()
                        }
                    }
                } else {
                    session_user.into()
                }
            }

            // Mount the file system
            main_mount(mount, conf, group, session, opts.no_auth).await?;
        }
    }

    info!("atefs::shutdown");

    Ok(())
}
