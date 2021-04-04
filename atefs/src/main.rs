#![allow(unused_imports, dead_code)]
use log::{info, error, debug};
use ate::prelude::*;
use std::env;
use std::io::ErrorKind;
use directories::BaseDirs;
use std::sync::Arc;
use std::ops::Deref;
use url::Url;

mod fixed;
mod api;
mod model;
mod dir;
mod symlink;
mod file;
mod progress;
mod fs;

use fs::AteFS;

use fuse3::raw::prelude::*;
use fuse3::{MountOptions};
use clap::Clap;

#[derive(Clap)]
#[clap(version = "0.1", author = "John S. <johnathan.sharratt@gmail.com>")]
struct Opts {
    /// Sets the level of log verbosity, can be used multiple times
    #[allow(dead_code)]
    #[clap(short, long, parse(from_occurrences))]
    verbose: i32,
    /// URL where the user is authenticated
    #[clap(short, long, default_value = "tcp://ate.tokera.com/auth")]
    auth: Url,
    /// Logs debug info to the console
    #[clap(short, long)]
    debug: bool,
    /// Determines if ATE will use DNSSec or just plain DNS
    #[clap(long)]
    dns_sec: bool,
    /// Address that DNS queries will be sent to
    #[clap(long, default_value = "8.8.8.8")]
    dns_server: String,
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    #[clap()]
    Mount(Mount),
    #[clap()]
    Solo(Solo),
    #[clap()]
    Login(Login),
    #[clap()]
    Logout(Logout),
}

/// Logs into the authentication server using the supplied credentials
#[derive(Clap)]
struct Login {
    /// Email address that you wish to login using
    #[clap(index = 1)]
    email: String,
    /// Password associated with this account
    #[clap(index = 2)]
    password: Option<String>,
    /// Authenticator code from your google authenticator
    #[clap(index = 3)]
    code: Option<String>
}

/// Logs out by removing all the authentication tokens from the local machine
#[derive(Clap)]
struct Logout {
}

/// Runs a solo ATE database and listens for connections from clients
#[derive(Clap)]
struct Solo {
    /// Path to the log files where all the file system data is stored
    #[clap(index = 1, default_value = "/opt/fs")]
    logs_path: String,
    /// IP address that the authentication server will isten on
    #[clap(short, long, default_value = "0.0.0.0")]
    listen: String,
    /// Port that the authentication server will listen on
    #[clap(short, long, default_value = "5000")]
    port: u16,
}

/// Mounts a particular directory as an ATE file system
#[derive(Clap)]
struct Mount {
    /// Path to directory that the file system will be mounted at
    #[clap(index=1)]
    mount_path: String,
    /// Location of the persistent redo log
    #[clap(index=2, default_value = "~/ate/fs")]
    log_path: String,
    /// URL where the data is remotely stored on a distributed commit log (e.g. tcp://ate.tokera.com/myfs).
    /// If this URL is not specified then data will only be stored locally
    #[clap(index=3)]
    remote: Option<Url>,
    /// Local redo log file will be deleted when the file system is unmounted, remotely stored data on
    /// any distributed commit log will be persisted. Effectively this setting only uses the local disk
    /// as a cache of the redo-log while it's being used.
    #[clap(short, long)]
    temp: bool,
    /// Indicates if ATE will use quantum resistant wire encryption (possible values are 128, 192, 256).
    #[clap(long)]
    wire_encryption: Option<KeySize>,
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
    #[clap(short, long)]
    non_empty: bool,
    /// Configure the log file for <raw>, <barebone>, <speed>, <compatibility>, <balanced> or <security>
    #[clap(long, default_value = "speed")]
    configured_for: ate::conf::ConfiguredFor,
    /// Format of the metadata in the log file as <bincode>, <json> or <mpack>
    #[clap(long, default_value = "bincode")]
    meta_format: ate::spec::SerializationFormat,
    /// Format of the data in the log file as <bincode>, <json> or <mpack>
    #[clap(long, default_value = "bincode")]
    data_format: ate::spec::SerializationFormat,
}

#[allow(dead_code)]
fn main_debug() -> Opts {
    Opts {
        verbose: 2,
        debug: true,
        dns_sec: false,
        dns_server: "8.8.8.8".to_string(),
        auth: Url::from_str("tcp://ate.tokera.com/auth").unwrap(),
        subcmd: SubCommand::Mount(Mount {
            mount_path: "/mnt/test".to_string(),
            log_path: "~/ate/fs".to_string(),
            remote: Some(Url::from_str("tcp://localhost/myfs").unwrap()),
            temp: false,
            uid: None,
            gid: None,
            wire_encryption: None,
            allow_root: false,
            allow_other: false,
            read_only: false,
            write_back: false,
            non_empty: false,
            configured_for: ate::conf::ConfiguredFor::BestPerformance,
            meta_format: ate::spec::SerializationFormat::Bincode,
            data_format: ate::spec::SerializationFormat::Bincode,
        }),
    }
}

fn ctrl_channel() -> tokio::sync::watch::Receiver<bool> {
    let (sender, receiver) = tokio::sync::watch::channel(false);
    ctrlc::set_handler(move || {
        let _ = sender.send(true);
    }).unwrap();
    receiver
}

#[tokio::main]
async fn main() -> Result<(), AteError> {
    let opts: Opts = Opts::parse();
    //let opts = main_debug();

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
    
    match opts.subcmd {
        SubCommand::Login(login) => {
            let _session = ate_auth::main_login(Some(login.email), login.password, login.code, opts.auth).await?;
        },
        SubCommand::Logout(logout) => {
            main_logout(logout).await?;
        },
        SubCommand::Mount(mount) => {
            main_mount(mount, conf).await?;
        },
        SubCommand::Solo(solo) => {
            main_solo(solo).await?;
        }
    }

    info!("atefs::shutdown");

    Ok(())
}

async fn main_solo(solo: Solo) -> Result<(), AteError>
{
    // Create the chain flow and generate configuration
    let mut cfg_ate = ate_auth::conf_auth();
    cfg_ate.log_path = shellexpand::tilde(&solo.logs_path).to_string();

    // Create the server and listen on port 5000
    let cfg_mesh = ConfMesh::solo(solo.listen.as_str(), solo.port);
    let _server = create_persistent_server(&cfg_ate, &cfg_mesh).await;

    // Wait for ctrl-c
    let mut exit = ctrl_channel();
    while *exit.borrow() == false {
        exit.changed().await.unwrap();
    }
    println!("Goodbye!");
    Ok(())
}

async fn main_logout(_logout: Logout) -> Result<(), AteError>
{
    panic!("Not implemented");
}

async fn main_mount(mount: Mount, conf: ConfAte) -> Result<(), AteError>
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
    conf.log_path = shellexpand::tilde(&mount.log_path).to_string();
    conf.wire_encryption = mount.wire_encryption;

    info!("configured_for: {:?}", mount.configured_for);
    info!("meta_format: {:?}", mount.meta_format);
    info!("data_format: {:?}", mount.data_format);
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

    // We create a chain with a specific key (this is used for the file name it creates)
    debug!("chain-init");
    let registry;
    let session;
    let chain = match mount.remote {
        None => {
            Arc::new(Chain::new(builder.clone(), &ChainKey::from("root")).await?)
        },
        Some(remote) => {
            registry = ate::mesh::Registry::new(&conf).await;
            session = registry.open_ext(&remote, Box::new(progress::LoadProgress::default())).await?;
            session.chain()
        },
    };
    
    // Mount the file system
    info!("mounting file-system and entering main loop");
    match Session::new(mount_options)
        .mount_with_unprivileged(AteFS::new(chain), mount.mount_path)
        .await
    {
        Err(err) if err.kind() == ErrorKind::Other => {
            if err.to_string().contains("find fusermount binary failed") {
                error!("Fuse3 could not be found - you may need to install fuse3 via apt/yum");
                return Ok(())
            }
            error!("{}", err);
            std::process::exit(1);
        }
        Err(err) => {
            error!("{}", err);
            std::process::exit(1);
        }
        _ => {}
    }

    Ok(())
}