#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use ate::prelude::*;
use url::Url;

use atefs::error::CommandError;
use atefs::opts::*;
use atefs::main_mount;
use ate::compact::CompactMode;

use ate_auth::cmd::*;
use ate_auth::helper::*;

use clap::Clap;

#[allow(dead_code)]
fn test_opts() -> Opts {
    Opts {
        verbose: 0,
        auth: Url::parse("ws://tokera.com/auth").unwrap(),
        no_auth: false,
        token: None,
        token_path: Some("~/token".to_string()),
        no_ntp: false,
        ntp_pool: None,
        ntp_port: None,
        debug: false,
        dns_sec: false,
        dns_server: "8.8.8.8".to_string(),
        subcmd: SubCommand::Mount(OptsMount {
            mount_path: "/mnt/ate".to_string(),
            remote: Url::parse("ws://tokera.com/db/").unwrap(),
            remote_name: Some("myfs".to_string()),
            log_path: Some("~/ate/fs".to_string()),
            backup_path: None,
            recovery_mode: RecoveryMode::ReadOnlyAsync,
            passcode: None,
            temp: false,
            uid: None,
            gid: None,
            allow_root: false,
            allow_other: false,
            read_only: false,
            write_back: false,
            non_empty: false,
            impersonate_uid: true,
            configured_for: ate::conf::ConfiguredFor::BestPerformance,
            meta_format: SerializationFormat::Bincode,
            data_format: SerializationFormat::Bincode,
            compact_now: false,
            compact_mode: CompactMode::Never,
            compact_timer: 3600,
            compact_threshold_factor: 0.2,
            compact_threshold_size: 104857600
        })
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), CommandError> {
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
    
    match opts.subcmd {
        SubCommand::Token(opts_token) => {
            main_opts_token(opts_token, opts.token, opts.token_path, opts.auth, "Group").await?;
        },
        SubCommand::User(opts_user) => {
            main_opts_user(opts_user, opts.token, opts.token_path, opts.auth).await?;
        },
        SubCommand::Group(opts_group) => {
            if opts.no_auth {
                eprintln!("In order to create groups you must use some form of authentication.");
                std::process::exit(1);
            }
            main_opts_group(opts_group, opts.token, opts.token_path, opts.auth, "Group").await?;
        },
        SubCommand::Mount(mount) =>
        {
            // Derive the group from the mount address
            let mut group = None;
            if let Some(remote) = &mount.remote_name {
                if let Some((group_str, _)) = remote.split_once("/") {
                    group = Some(group_str.to_string());
                }
            }

            
            let mut session: AteSessionType = AteSessionUser::default().into();
            
            // If a passcode is supplied then use this
            if let Some(pass) = &mount.passcode
            {
                if opts.token.is_some() || opts.token_path.is_some() {
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
                let session_user = main_session_user(opts.token.clone(), opts.token_path.clone(), Some(opts.auth.clone())).await?;
                
                // Attempt to grab additional permissions for the group (if it has any)
                session = if group.is_some() {
                    match main_gather(group.clone(), session_user.clone().into(), opts.auth, "Group").await {
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
        },
    }

    info!("atefs::shutdown");

    Ok(())
}