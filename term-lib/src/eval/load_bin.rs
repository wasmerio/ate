#![allow(dead_code)]
#![allow(unused)]
use bytes::Bytes;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use std::collections::HashSet;

use super::EvalContext;
use crate::err::*;
use crate::fs::*;
use crate::stdio::*;
use crate::wasmer_vfs::FileSystem;
use super::BinaryPackage;

pub async fn load_bin(
    ctx: &mut EvalContext,
    name: &String,
    stdio: &mut Stdio,
) -> Option<BinaryPackage>
{
    // Resolve any alias
    let mut already = HashSet::<String>::default();
    let mut cmd = name.clone();
    while let Ok(mut file) = ctx
        .root
        .new_open_options()
        .read(true)
        .open(format!("/bin/{}.alias", cmd))
    {
        // Infinite loop check
        if already.contains(&cmd) {
            return None;
        }
        already.insert(cmd.clone());

        let mut d = Vec::new();
        if let Ok(_) = file.read_to_end(&mut d) {
            let next = String::from_utf8_lossy(&d[..]).to_string();
            debug!("binary alias '{}' found for {}", next, cmd);
            cmd = next;
        } else {
            break;
        }
    }

    // Check if there is a file in the /bin and /wapm_packages/.bin folder
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
                return Some(BinaryPackage::new(d));
            }
        }
    }

    // Fetch the data asynchronously (from the web site)
    ctx.bins.get(cmd.as_str(), stdio.stderr.clone())
        .await
}