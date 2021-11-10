#![allow(dead_code)]
#![allow(unused)]
#[allow(unused_imports, dead_code)]
use tracing::{info, error, debug, trace, warn};
use bytes::Bytes;
use wasmer_wasi::vfs::FileSystem;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

use super::EvalContext;
use crate::fs::*;
use crate::stdio::*;
use crate::err::*;

pub async fn load_bin
(
    ctx: &mut EvalContext,
    cmd: &String,
    stdio: &mut Stdio,
) -> Option<(Bytes, TmpFileSystem)>
{
    // Check if there is a file in the /bin and /wapm_packages/.bin folder
    let mut data = None;
    let file_checks = vec! [
        format!("/bin/{}", cmd),
        format!("/{}", cmd),
        format!("{}", cmd),
    ];
    for file_check in file_checks {
        if let Ok(mut file) = stdio.root.new_open_options().read(true).open(file_check) {
            let mut d = Vec::new();
            if let Ok(_) = file.read_to_end(&mut d) {
                data = Some(Bytes::from(d));
                break;
            }
        }
    }

    // Search for in the wapm_packages
    if data.is_none() {
        let search_path = if cmd.contains("/") {
            format!("/wapm_packages/{}@", cmd)
        } else {
            format!("/wapm_packages/_/{}@", cmd)
        };
        if let Ok(file) = stdio.root.search(&Path::new("/wapm_packages"), Some(search_path.as_str()), Some(".wasm")) {
            if let Ok(mut file) = stdio.root.new_open_options().read(true).open(file).map_err(|_e| ERR_ENOENT) {
                let mut d = Vec::new();
                if let Ok(_) = file.read_to_end(&mut d) {
                    data = Some(Bytes::from(d));
                }
            }
        }
    }

    // Fetch the data asynchronously (from the web site)
    if data.is_none() {
        data = ctx.bins.get(cmd).await;
    }

    // Grab the private file system for this binary (if the binary changes the private
    // file system will also change)
    match data {
        Some(data) => {
            let fs_private = ctx.bins.fs(&data).await;
            Some((data, fs_private))
        },
        None => {
            None
        }
    }
}