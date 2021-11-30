#![allow(dead_code)]
#![allow(unused)]
use bytes::Bytes;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use sha2::{Digest, Sha256};

use super::EvalContext;
use crate::err::*;
use crate::fs::*;
use crate::stdio::*;
use crate::wasmer_vfs::FileSystem;

pub async fn load_bin(
    ctx: &mut EvalContext,
    cmd: &String,
    stdio: &mut Stdio,
) -> Option<(String, Bytes, TmpFileSystem)> {
    // Check if there is an alias
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
        file_checks.push(format!("{}{}", ctx.path, &cmd[2..]));
    }
    for file_check in file_checks {
        if let Ok(mut file) = ctx.root.new_open_options().read(true).open(file_check) {
            let mut d = Vec::new();
            if let Ok(_) = file.read_to_end(&mut d) {
                let d = Bytes::from(d);
                data = Some((hash_of_binary(&d), d));
                break;
            }
        }
    }

    // Fetch the data asynchronously (from the web site)
    if data.is_none() {
        let d = ctx.bins.get(cmd.as_str(), stdio.stderr.clone()).await;
        if let Some(d) = d {
            data = Some((hash_of_binary(&d), d));
        }
    }

    // Grab the private file system for this binary (if the binary changes the private
    // file system will also change)
    match data {
        Some((hash, data)) => {
            let fs_private = ctx.bins.fs(&hash).await;
            Some((hash, data, fs_private))
        }
        None => None,
    }
}

fn hash_of_binary(data: &Bytes) -> String
{
    let mut hasher = Sha256::default();
    hasher.update(data.as_ref());
    let hash = hasher.finalize();
    base64::encode(&hash[..])
}