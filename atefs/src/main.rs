#![allow(unused_imports, dead_code)]
use log::{info, error, debug};
use ate::prelude::*;
use ate_auth::prelude::*;
use ate_auth::opts::*;
use std::env;
use std::io::ErrorKind;
use directories::BaseDirs;
use std::sync::Arc;
use std::ops::Deref;
use std::time::Duration;
use url::Url;
use tokio::select;

mod fixed;
mod api;
mod model;
mod dir;
mod symlink;
mod file;
mod progress;
mod error;
mod fs;
mod umount;

use crate::fs::AteFS;
use crate::error::CommandError;
use ate::compact::CompactMode;

use fuse3::raw::prelude::*;
use fuse3::{MountOptions};
use clap::Clap;

#[derive(Clap)]
#[clap(version = "1.3", author = "John S. <johnathan.sharratt@gmail.com>")]
struct Opts {
    /// Sets the level of log verbosity, can be used multiple times
    #[allow(dead_code)]
    #[clap(short, long, parse(from_occurrences))]
    verbose: i32,
    /// URL where the user is authenticated
    #[clap(short, long, default_value = "tcp://auth.tokera.com:5001/auth")]
    auth: Url,
    /// No authentication or passcode will be used to protect this file-system
    #[clap(short, long)]
    no_auth: bool,
    /// Token used to access your encrypted file-system (if you do not supply a token then you will
    /// be prompted for a username and password)
    #[clap(short, long)]
    token: Option<String>,
    /// Token file to read that holds a previously created token to be used to access your encrypted
    /// file-system (if you do not supply a token then you will be prompted for a username and password)
    #[clap(long)]
    token_path: Option<String>,
    /// No NTP server will be used to synchronize the time thus the server time
    /// will be used instead
    #[clap(long)]
    no_ntp: bool,
    /// NTP server address that the file-system will synchronize with
    #[clap(long)]
    ntp_pool: Option<String>,
    /// NTP server port that the file-system will synchronize with
    #[clap(long)]
    ntp_port: Option<u16>,
    /// Logs debug info to the console
    #[clap(short, long)]
    debug: bool,
    /// Determines if ATE will use DNSSec or just plain DNS
    #[clap(long)]
    dns_sec: bool,
    /// Address that DNS queries will be sent to
    #[clap(long, default_value = "8.8.8.8")]
    dns_server: String,
    /// Indicates if ATE will use quantum resistant wire encryption (possible values are 128, 192, 256).
    /// The default is not to use wire encryption meaning the encryption of the event data itself is
    /// what protects the data
    #[clap(long)]
    wire_encryption: Option<KeySize>,
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    /// Mounts a local or remote file system
    #[clap()]
    Mount(Mount),
    /// Users are needed to access any remote file systems
    #[clap()]
    User(OptsUser),
    /// Groups are collections of users that share same remote file system
    #[clap()]
    Group(OptsGroup),
    /// Tokens are needed to mount file systems without prompting for credentials
    #[clap()]
    Token(OptsToken),
}

/// Mounts a particular directory as an ATE file system
#[derive(Clap)]
struct Mount {
    /// Path to directory that the file system will be mounted at
    #[clap(index=1)]
    mount_path: String,
    /// URL where the data is remotely stored on a distributed commit log (e.g. tcp://ate.tokera.com/).
    /// If this URL is not specified then data will only be stored locally
    #[clap(index=2)]
    remote: Option<Url>,
    /// Location of the persistent redo log
    #[cfg_attr(feature = "local_fs", clap(index=3, default_value = "~/ate/fs"))]
    #[cfg_attr(not(feature = "local_fs"), clap(long = "_unused_logs_path", hidden = true, default_value = "/_unused_path/"))]
    #[allow(dead_code)]
    log_path: String,
    /// Determines how the file-system will react while it is nominal and when it is
    /// recovering from a communication failure (valid options are 'async', 'readonly-async',
    /// 'readonly-sync' or 'sync')
    #[clap(long, default_value = "readonly-async")]
    recovery_mode: RecoveryMode,
    /// User supplied passcode that will be used to encrypt the contents of this file-system
    /// instead of using an authentication. Note that this can 'not' be used as combination
    /// with a strong authentication system and hence implicitely implies the 'no-auth' option
    /// as well.
    #[clap(short, long)]
    passcode: Option<String>,
    /// Local redo log file will be deleted when the file system is unmounted, remotely stored data on
    /// any distributed commit log will be persisted. Effectively this setting only uses the local disk
    /// as a cache of the redo-log while it's being used.
    #[clap(long)]
    temp: bool,
    /// UID of the user that this file system will be mounted as
    #[clap(short, long)]
    uid: Option<u32>,
    /// GID of the group that this file system will be mounted as
    #[clap(short, long)]
    gid: Option<u32>,
    /// Allow the root user to have access to this file system
    #[clap(long)]
    allow_root: bool,
    /// Allow other users on the machine to have access to this file system
    #[clap(long)]
    allow_other: bool,
    /// Mount the file system in readonly mode (`ro` mount option), default is disable.
    #[clap(short, long)]
    read_only: bool,
    /// Enable write back cache for buffered writes, default is disable.
    #[clap(short, long)]
    write_back: bool,
    /// Allow fuse filesystem mount on a non-empty directory, default is not allowed.
    #[clap(long)]
    non_empty: bool,
    /// For files and directories that the authenticated user owns, translate the UID and GID to the local machine ids instead of the global ones.
    #[clap(short, long)]
    impersonate_uid: bool, 
    /// Configure the log file for <raw>, <barebone>, <speed>, <compatibility>, <balanced> or <security>
    #[clap(long, default_value = "speed")]
    configured_for: ate::conf::ConfiguredFor,
    /// Format of the metadata in the log file as <bincode>, <json> or <mpack>
    #[clap(long, default_value = "bincode")]
    meta_format: ate::spec::SerializationFormat,
    /// Format of the data in the log file as <bincode>, <json> or <mpack>
    #[clap(long, default_value = "bincode")]
    data_format: ate::spec::SerializationFormat,
    /// Forces the compaction of the local redo-log before it streams in the latest values
    #[clap(long)]
    compact_now: bool,
    /// Mode that the compaction will run under (valid modes are 'never', 'modified', 'timer', 'factor', 'size', 'factor-or-timer', 'size-or-timer')
    #[clap(long, default_value = "factor-or-timer")]
    compact_mode: CompactMode,
    /// Time in seconds between compactions of the log file (default: 1 hour) - this argument is ignored if you select a compact_mode that has no timer
    #[clap(long, default_value = "3600")]
    compact_timer: u64,
    /// Factor growth in the log file which will trigger compaction (default: 0.2) - this argument is ignored if you select a compact_mode that has no growth trigger
    #[clap(long, default_value = "0.2")]
    compact_threshold_factor: f32,
    /// Size of growth in bytes in the log file which will trigger compaction (default: 100MB) - this argument is ignored if you select a compact_mode that has no growth trigger
    #[clap(long, default_value = "104857600")]
    compact_threshold_size: u64,
}

fn ctrl_channel() -> tokio::sync::watch::Receiver<bool> {
    let (sender, receiver) = tokio::sync::watch::channel(false);
    ctrlc::set_handler(move || {
        let _ = sender.send(true);
    }).unwrap();
    receiver
}

async fn main_mount(mount: Mount, conf: ConfAte, group: Option<String>, session: AteSession, no_auth: bool) -> Result<(), AteError>
{
    let uid = match mount.uid {
        Some(a) => a,
        None => unsafe { libc::getuid() }
    };
    let gid = match mount.gid {
        Some(a) => a,
        None => unsafe { libc::getgid() }
    };

    debug!("uid: {}", uid);
    debug!("gid: {}", uid);

    let mount_options = MountOptions::default()
        .uid(uid)
        .gid(gid)
        .allow_root(mount.allow_root)
        .allow_other(mount.allow_other)
        .read_only(mount.read_only)
        .write_back(mount.write_back)
        .nonempty(mount.non_empty);

    debug!("allow_root: {}", mount.allow_root);
    debug!("allow_other: {}", mount.allow_other);
    debug!("read_only: {}", mount.read_only);
    debug!("write_back: {}", mount.write_back);
    debug!("non_empty: {}", mount.non_empty);
    
    let mut conf = conf.clone();
    conf.configured_for(mount.configured_for);
    conf.log_format.meta = mount.meta_format;
    conf.log_format.data = mount.data_format;
    #[cfg(feature = "local_fs")]
    {
        conf.log_path = shellexpand::tilde(&mount.log_path).to_string();
    }
    conf.recovery_mode = mount.recovery_mode;
    conf.compact_bootstrap = mount.compact_now;
    conf.compact_mode = mount.compact_mode
        .with_growth_factor(mount.compact_threshold_factor)
        .with_growth_size(mount.compact_threshold_size)
        .with_timer_value(Duration::from_secs(mount.compact_timer));

    info!("configured_for: {:?}", mount.configured_for);
    info!("meta_format: {:?}", mount.meta_format);
    info!("data_format: {:?}", mount.data_format);
    #[cfg(feature = "local_fs")]
    info!("log_path: {}", conf.log_path);
    info!("log_temp: {}", mount.temp);
    info!("mount_path: {}", mount.mount_path);
    match &mount.remote {
        Some(remote) => info!("remote: {}", remote.to_string()),
        None => info!("remote: local-only"),
    };

    let builder = ChainBuilder::new(&conf)
        .await
        .temporal(mount.temp);

    // Create a progress bar loader
    let mut progress_local = Box::new(progress::LoadProgress::default());
    let mut progress_remote = Box::new(progress::LoadProgress::default());
    progress_local.units = pbr::Units::Bytes;
    progress_local.msg_done = "Downloading latest events from server...".to_string();
    progress_remote.msg_done = "Loaded the remote chain-of-trust, proceeding to mount the file system.".to_string();
    eprint!("Loading the chain-of-trust...");

    // We create a chain with a specific key (this is used for the file name it creates)
    debug!("chain-init");
    let registry;
    let chain = match mount.remote {
        None => {
            Arc::new(
                Chain::new_ext(
                    builder.clone(),
                    ChainKey::from("root"),
                    Some(progress_local),
                    true
                ).await?
            )
        },
        Some(remote) => {
            registry = ate::mesh::Registry::new(&conf, mount.temp).await;
            registry.open_ext(&remote, progress_local, progress_remote).await?
        },
    };

    // Compute the scope
    let scope = match mount.recovery_mode.is_sync() {
        true => TransactionScope::Full,
        false => TransactionScope::Local,
    };

    // Create the mount point
    let mount_path = mount.mount_path.clone();
    let mount_join = Session::new(mount_options)
        .mount_with_unprivileged(AteFS::new(chain, group, session, scope, no_auth, mount.impersonate_uid), mount.mount_path);

    // Install a ctrl-c command
    info!("mounting file-system and entering main loop");
    let mut ctrl_c = ctrl_channel();

    // Add a panic hook that will unmount
    {
        let orig_hook = std::panic::take_hook();
        let mount_path = mount_path.clone();
        std::panic::set_hook(Box::new(move |panic_info| {
            let _ = umount::unmount(std::path::Path::new(mount_path.as_str()));
            orig_hook(panic_info);
            std::process::exit(1);
        }));
    }

    // Main loop
    eprintln!("Press ctrl-c to exit");
    select!
    {
        // Wait for a ctrl-c
        _ = ctrl_c.changed() => {
            umount::unmount(std::path::Path::new(mount_path.as_str()))?;
            eprintln!("Goodbye!");
            return Ok(());
        }

        // Mount the file system
        ret = mount_join => {
            match ret {
                Err(err) if err.kind() == ErrorKind::Other => {
                    if err.to_string().contains("find fusermount binary failed") {
                        error!("Fuse3 could not be found - you may need to install fuse3 via apt/yum");
                        return Ok(())
                    }
                    error!("{}", err);
                    let _ = umount::unmount(std::path::Path::new(mount_path.as_str()));
                    std::process::exit(1);
                }
                Err(err) => {
                    error!("{}", err);
                    let _ = umount::unmount(std::path::Path::new(mount_path.as_str()));
                    std::process::exit(1);
                }
                _ => {
                    eprintln!("Shutdown");
                    let _ = umount::unmount(std::path::Path::new(mount_path.as_str()));
                    return Ok(());
                }
            }
        }
    }
}

fn test_opts() -> Opts {
    Opts {
        verbose: 0,
        auth: Url::parse("tcp://auth.tokera.com:5001/auth").unwrap(),
        no_auth: false,
        token: None,
        token_path: Some("~/token".to_string()),
        no_ntp: false,
        ntp_pool: None,
        ntp_port: None,
        debug: false,
        dns_sec: false,
        dns_server: "8.8.8.8".to_string(),
        wire_encryption: None,
        subcmd: SubCommand::Mount(Mount {
            mount_path: "/mnt/ate".to_string(),
            remote: Some(Url::parse("tcp://ate.tokera.com/").unwrap()),
            log_path: "~/ate/fs".to_string(),
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

#[tokio::main]
async fn main() -> Result<(), CommandError> {
    let opts: Opts = Opts::parse();
    //let opts = test_opts();
    
    let mut log_level = match opts.verbose {
        0 => "error",
        1 => "warn",
        2 => "info",
        _ => "debug",
    };
    if opts.debug { log_level = "debug"; }
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    let mut conf = AteConfig::default();
    conf.dns_sec = opts.dns_sec;
    conf.dns_server = opts.dns_server;
    conf.ntp_sync = opts.no_ntp == false;
    conf.wire_encryption = opts.wire_encryption;
    
    if let Some(pool) = opts.ntp_pool {
        conf.ntp_pool = pool;
    }
    if let Some(port) = opts.ntp_port {
        conf.ntp_port = port;
    }
    
    match opts.subcmd {
        SubCommand::Token(opts_token) => {
            ate_auth::main_opts_token(opts_token, opts.token, opts.token_path, opts.auth).await?;
        },
        SubCommand::User(opts_user) => {
            ate_auth::main_opts_user(opts_user, opts.token, opts.token_path, opts.auth).await?;
        },
        SubCommand::Group(opts_group) => {
            if opts.no_auth {
                eprintln!("In order to create groups you must use some form of authentication.");
                std::process::exit(1);
            }
            ate_auth::main_opts_group(opts_group, opts.token, opts.token_path, opts.auth).await?;
        },
        SubCommand::Mount(mount) =>
        {
            // Create a default empty session
            let mut group = None;
            let mut session = AteSession::default();

            // If a passcode is supplied then use this
            if let Some(pass) = &mount.passcode
            {
                if opts.token.is_some() || opts.token_path.is_some() {
                    eprintln!("You can not supply both a passcode and a token, either drop the --token arguments or the --passcode argument");
                    std::process::exit(1);
                }
                if mount.remote.is_some() {
                    eprintln!("Using a passcode is not compatible with remotely hosted file-systems as the distributed databases need to make authentication checks");
                    std::process::exit(1);
                }

                let prefix = "ate:".to_string();
                let key = ate_auth::password_to_read_key(&prefix, &pass, 15, KeySize::Bit192);
                session.user.add_read_key(&key);

            } else if opts.no_auth {
                if mount.remote.is_some() {
                    eprintln!("In order to use remotely hosted file-systems you must use some form of authentication, without authentication the distributed databases will not be able to make the needed checks");
                    std::process::exit(1);
                }

                // We do not put anything in the session as no authentication method nor a passcode was supplied
            } else {
                // Load the session via the token or the authentication server
                session = ate_auth::main_session(opts.token.clone(), opts.token_path.clone(), Some(opts.auth.clone()), false).await?;

                // Attempt to grab additional permissions for the group (if it has any)
                if let Some(remote) = &mount.remote {
                    group = Some(remote.path().to_string());
                    session = match ate_auth::main_gather(group.clone(), session.clone(), opts.auth).await {
                        Ok(a) =>
                        {
                            a
                        },
                        Err(err) => {
                            debug!("Group authentication failed: {} - falling back to user level authorization", err);
                            session
                        }
                    };
                }
            }

            // Mount the file system
            main_mount(mount, conf, group, session, opts.no_auth).await?;
        },
    }

    info!("atefs::shutdown");

    Ok(())
}