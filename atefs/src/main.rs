#![allow(unused_imports)]
use log::{info, error};
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

#[tokio::main]
async fn main() -> Result<(), AteError> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);

    let args = env::args_os().skip(1).take(1).collect::<Vec<_>>();

    let mount_path = args.first();

    let uid = unsafe { libc::getuid() };
    let gid = unsafe { libc::getgid() };

    let mount_options = MountOptions::default().uid(uid).gid(gid);
    let mount_path = mount_path.expect("no mount point specified");

    // The default configuration will store the redo log locally in the temporary folder
    let mut conf = AteConfig::default();
    conf.configured_for(ConfiguredFor::BestPerformance);
    //conf.log_format.meta = ate::spec::SerializationFormat::Json;
    //conf.log_format.data = ate::spec::SerializationFormat::Json;
    
    conf.log_path = BaseDirs::new().unwrap().home_dir().join("ate").to_str().unwrap().to_string();
    conf.log_temp = false;
    let builder = ChainBuilder::new(&conf);

    // We create a chain with a specific key (this is used for the file name it creates)
    info!("atefs::chain-init");
    let chain = Chain::new(builder.clone(), &ChainKey::from("root")).await?;
    
    // Mount the file system
    info!("atefs::mount");
    match Session::new(mount_options)
        .mount_with_unprivileged(AteFS::new(chain), mount_path)
        .await
    {
        Err(err) if err.kind() == ErrorKind::Other => {
            if err.to_string().contains("find fusermount binary failed") {
                error!("Fuse3 could not be found - you may need to install apt install fuse3");
                return Ok(())
            }
            panic!("{}", err);
        }
        _ => {}
    }

    info!("atefs::shutdown");

    Ok(())
}