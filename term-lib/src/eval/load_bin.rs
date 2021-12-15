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
    while let Ok(mut file) = AsyncifyFileSystem::new(ctx.root.clone())
        .new_open_options()
        .await
        .read(true)
        .open(format!("/bin/{}.alias", name))
        .await
    {
        if already.contains(&name) {
            break;
        }
        already.insert(name.clone());

        if let Ok(d) = file.read_to_end().await {
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
        file_checks.push(format!("{}{}", ctx.working_dir, &name[2..]));
    }
    for file_check in file_checks {
        if let Ok(mut file) = AsyncifyFileSystem::new(ctx.root.clone())
            .new_open_options()
            .await
            .read(true)
            .open(file_check)
            .await
        {
            if let Ok(d) = file.read_to_end().await {
                let d = Bytes::from(d);
                return Some(BinaryPackage::new(d));
            }
        }
    }

    // Resolve some more alias possibilities using fetch commands (with cached results)
    let mut chroot = false;
    while let Some(next) = ctx.bins.alias(name.as_str(), stdio.stderr.clone()).await {
        if already.contains(&name) {
            break;
        }
        already.insert(name.clone());
        if next.chroot {
            chroot = true;
        }
        name = next.alias;
    }

    // Fetch the data asynchronously (from the web site)
    let mut ret = ctx.bins.get(name.as_str(), stdio.stderr.clone()).await;
    if let Some(ret) = ret.as_mut() {
        if chroot {
            ret.chroot = true;
        }
    }
    ret
}
