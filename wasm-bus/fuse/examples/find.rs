use std::{collections::VecDeque, path::Path};
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use tracing_subscriber::fmt::SubscriberBuilder;
use wasm_bus_fuse::prelude::*;
//use tracing_subscriber::EnvFilter;
use tracing::metadata::LevelFilter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    wasm_bus::task::block_on(main_async())
}

async fn main_async() -> Result<(), Box<dyn std::error::Error>> {
    SubscriberBuilder::default()
        .with_writer(std::io::stderr)
        .with_max_level(LevelFilter::DEBUG)
        //.with_env_filter(EnvFilter::from_default_env())
        .init();

    let args: Vec<String> = std::env::args().collect();
    let program = args[0].clone();

    if args.len() != 3 && args.len() != 5 {
        eprintln!(
            "usage: {} <db-name> <filename> [instance] [access-code]",
            program
        );
        return Ok(());
    }
    let name = args[1].clone();
    let file = args[2].clone();

    let fs = if args.len() == 5 {
        let instance = args[3].clone();
        let access_code = args[4].clone();
        FileSystem::mount_instance(instance.as_str(), access_code.as_str(), "tok", &name)
            .await
            .map_err(conv_err)?
    } else {
        FileSystem::mount("tok", &name).await.map_err(conv_err)?
    };

    find(&fs, "/", &file).await.map_err(conv_err)?;

    Ok(())
}

async fn find(fs: &FileSystem, path: &str, file: &str) -> FsResult<()> {
    let mut work = VecDeque::new();
    work.push_back(path.to_string());

    while let Some(path) = work.pop_front() {
        let dir = fs.read_dir(Path::new(path.as_str())).await?;
        for entry in dir.data {
            if let Some(meta) = entry.metadata {
                if meta.ft.file && entry.path == file {
                    println!("{}{}", path, entry.path);
                }
                if meta.ft.dir {
                    let sub = format!("{}{}/", path, entry.path);
                    work.push_back(sub);
                }
            }
        }
    }
    Ok(())
}

fn conv_err(err: FsError) -> Box<dyn std::error::Error> {
    error!("{}", err);
    let err: std::io::Error = err.into();
    err.into()
}
