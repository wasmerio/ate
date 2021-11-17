#![allow(dead_code)]
#![allow(unused)]
use bytes::Bytes;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasmer_wasi::vfs::FileSystem;

use super::EvalContext;
use crate::err::*;
use crate::fs::*;
use crate::stdio::*;

pub async fn load_bin(
    ctx: &mut EvalContext,
    cmd: &String,
    stdio: &mut Stdio,
) -> Option<(Bytes, TmpFileSystem)> {
    // Check if there is an alias
    let _ = stdio.stderr.write("Searching...".as_bytes()).await;
    let mut cmd = cmd.clone();
    if let Ok(mut file) = ctx
        .root
        .new_open_options()
        .read(true)
        .open(format!("/bin/{}.alias", cmd))
    {
        let mut d = Vec::new();
        if let Ok(_) = file.read_to_end(&mut d) {
            cmd = String::from_utf8_lossy(&d[..]).to_string();
        }
    }

    // Check if there is a file in the /bin and /wapm_packages/.bin folder
    let mut data = None;
    let mut file_checks = vec![format!("/bin/{}", cmd)];
    if cmd.starts_with("/") {
        file_checks.push(cmd.clone());
    } else if cmd.starts_with("./") && cmd.len() > 2 {
        let state = ctx.console.lock().unwrap();
        file_checks.push(format!("{}{}", state.path, &cmd[2..]));
    }
    for file_check in file_checks {
        if let Ok(mut file) = ctx.root.new_open_options().read(true).open(file_check) {
            stdio.stderr.write_clear_line().await;
            let _ = stdio.stderr.write("Loading...".as_bytes()).await;

            let mut d = Vec::new();
            if let Ok(_) = file.read_to_end(&mut d) {
                data = Some(Bytes::from(d));
                break;
            }
        }
    }

    // Fetch the data asynchronously (from the web site)
    if data.is_none() {
        stdio.stderr.write_clear_line().await;
        let _ = stdio.stderr.write("Fetching...".as_bytes()).await;

        data = ctx.bins.get(cmd.as_str()).await;
    }

    // Grab the private file system for this binary (if the binary changes the private
    // file system will also change)
    stdio.stderr.write_clear_line().await;
    match data {
        Some(data) => {
            let fs_private = ctx.bins.fs(&data).await;
            Some((data, fs_private))
        }
        None => None,
    }
}
