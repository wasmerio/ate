#![allow(unused_imports)]
use log::{info, error, debug};
use ate::prelude::*;
use std::env;
use std::io::ErrorKind;
use directories::BaseDirs;

mod fixed;
mod api;
mod model;
mod dir;
mod symlink;
mod file;
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
    /// Logs debug info to the console
    #[allow(dead_code)]
    #[clap(short, long)]
    debug: bool,
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    #[clap(version = "1.0", author = "John S. <johnathan.sharratt@gmail.com>")]
    Mount(Mount),
}

/// Mounts a particular directory as an ATE file system
#[derive(Clap)]
struct Mount {
    /// Configuration file of a ATE mesh to connect to - default is local-only mode
    #[allow(dead_code)]
    #[clap(short, long)]
    mesh: Option<String>,
    /// Path to directory that the file system will be mounted at
    #[clap(short, long)]
    path: String,
    /// Location of the persistent redo log
    #[clap(short, long, default_value = "~/ate")]
    log: String,
    /// Redo log file will be deleted when the file system is unmounted
    #[clap(short, long)]
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
    #[clap(short, long)]
    non_empty: bool,
    /// Log file is configured for <raw>, <barebone>, <speed>, <compatibility>, <balanced> or <security>
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
        subcmd: SubCommand::Mount(Mount {
            mesh: None,
            path: "/mnt/test".to_string(),
            log: "~/ate".to_string(),
            temp: false,
            uid: None,
            gid: None,
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

#[tokio::main]
async fn main() -> Result<(), AteError> {
    let opts: Opts = Opts::parse();
    //let opts = main_debug();

    let log_level = match opts.verbose {
        1 => "info",
        2 => "debug",
        _ => "error",
    };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();
    
    match opts.subcmd {
        SubCommand::Mount(mount) => {
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
            
            let mut conf = AteConfig::default();
            conf.configured_for(mount.configured_for);
            conf.log_format.meta = mount.meta_format;
            conf.log_format.data = mount.data_format;
            conf.log_path = shellexpand::tilde(&mount.log).to_string();
            conf.log_temp = mount.temp;

            debug!("configured_for: {:?}", mount.configured_for);
            debug!("meta_format: {:?}", mount.meta_format);
            debug!("data_format: {:?}", mount.data_format);
            debug!("log_path: {}", conf.log_path);
            debug!("log_temp: {}", mount.temp);
            debug!("mount_path: {}", mount.path);

            let builder = ChainBuilder::new(&conf);

            // We create a chain with a specific key (this is used for the file name it creates)
            info!("atefs::chain-init");
            let chain = Chain::new(builder.clone(), &ChainKey::from("root")).await?;
            
            // Mount the file system
            info!("atefs::mount");
            match Session::new(mount_options)
                .mount_with_unprivileged(AteFS::new(chain), mount.path)
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
        }
    }

    info!("atefs::shutdown");

    Ok(())
}