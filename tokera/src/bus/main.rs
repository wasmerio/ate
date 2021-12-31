#![allow(unused_imports, dead_code)]
use crate::opt::OptsBus;
use ate::prelude::*;
use ate_auth::prelude::*;
use ate_files::codes::*;
use ate_files::prelude::*;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::watch;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::CallHandle;
use wasm_bus::abi::SerializationFormat;
use wasm_bus::task::listen;
use wasm_bus::task::respond_to;
use wasm_bus::task::serve;
use wasm_bus_fuse::api;
use wasm_bus_fuse::api::FuseService;
use wasm_bus_fuse::prelude::*;

use super::fuse::FuseServer;

pub async fn main_opts_bus(
    opts: OptsBus,
    conf: AteConfig,
    token_path: String,
    auth_url: url::Url,
) -> Result<(), crate::error::BusError> {
    info!("wasm bus initializing");

    // Start the fuse implementation
    FuseServer::serve(opts, conf, token_path, auth_url).await?;
    Ok(())
}

fn conv_file_type(kind: ate_files::api::FileKind) -> api::FileType {
    let mut ret = api::FileType::default();
    match kind {
        ate_files::api::FileKind::Directory => {
            ret.dir = true;
        }
        ate_files::api::FileKind::RegularFile => {
            ret.file = true;
        }
        ate_files::api::FileKind::FixedFile => {
            ret.file = true;
        }
        ate_files::api::FileKind::SymLink => {
            ret.symlink = true;
        }
    }
    ret
}

fn conv_meta(file: ate_files::attr::FileAttr) -> api::Metadata {
    api::Metadata {
        ft: conv_file_type(file.kind),
        accessed: file.accessed,
        created: file.created,
        modified: file.updated,
        len: file.size,
    }
}
