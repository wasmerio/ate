#![allow(dead_code)]
#![allow(unused)]
use bytes::Bytes;
use std::collections::HashSet;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use super::BinaryPackage;
use super::EvalContext;
use crate::err::*;
use crate::fs::*;
use crate::stdio::*;
use crate::wasmer_vfs::FileSystem;

pub async fn load_bin(
    ctx: &mut EvalContext,
    name: &String,
    stdio: &mut Stdio,
) -> Option<BinaryPackage> {
    // Resolve any alias
    let mut already = HashSet::<String>::default();
    let mut name = name.clone();
    debug!("scanning for {}", format!("/bin/{}.alias", name));
    while let Ok(mut file) = ctx
        .root
        .new_open_options()
        .read(true)
        .open(format!("/bin/{}.alias", name))
    {
        if already.contains(&name) {
            break;
        }
        already.insert(name.clone());

        let mut d = Vec::new();
        if let Ok(_) = file.read_to_end(&mut d) {
            let next = String::from_utf8_lossy(&d[..]).trim().to_string();
            info!("binary alias '{}' found for {}", next, name);
            name = next;
        } else {
            break;
        }
    }

    // Check if there is a file in the /bin and /wapm_packages/.bin folder
    let mut file_checks = vec![format!("/bin/{}", name)];
    if name.starts_with("/") {
        file_checks.push(name.clone());
    } else if name.starts_with("./") && name.len() > 2 {
        file_checks.push(format!("{}{}", ctx.path, &name[2..]));
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

    // Resolve some more alias possibilities using fetch commands
    while let Some(next) = ctx.bins.alias(name.as_str(), stdio.stderr.clone()).await {
        if already.contains(&name) {
            break;
        }
        already.insert(name.clone());
        name = next;
    }

    // Fetch the data asynchronously (from the web site)
    ctx.bins.get(name.as_str(), stdio.stderr.clone()).await
}
