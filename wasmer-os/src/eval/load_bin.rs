#![allow(dead_code)]
#![allow(unused)]
use bytes::Bytes;
use std::collections::HashSet;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use super::AliasConfig;
use super::BinaryPackage;
use super::EvalContext;
use crate::err::*;
use crate::fs::*;
use crate::stdio::*;
use crate::wasmer_vfs::FileSystem;
use crate::pipe::*;
use crate::fd::FdFlag;

pub async fn load_bin(
    ctx: &EvalContext,
    name: &String,
    stdio: &mut Stdio,
) -> Option<BinaryPackage> {
    // Resolve any alias
    let mut chroot = false;
    let mut wapm = None;
    let mut base_dir = None;
    let mut envs = HashMap::default();
    let mut mappings = Vec::new();
    let mut already = HashSet::<String>::default();
    let mut name = name.clone();

    // Enter a loop that will resolve aliases into real files
    let mut alias_loop = true;
    while alias_loop {
        // Build the list of alias paths
        let mut alias_checks = vec![
            format!("/bin/{}.alias", name),
            format!("/usr/bin/{}.alias", name),
        ];
        if name.starts_with("/") {
            alias_checks.push(format!("{}.alias", name));
        } else if name.starts_with("./") && name.len() > 2 {
            alias_checks.push(format!("{}{}.alias", ctx.working_dir, &name[2..]));
        }

        // If an alias file exists then process it...otherwise break from the loop
        alias_loop = false;
        for alias_check in alias_checks {
            debug!("scanning for {}", alias_check);
            if let Ok(mut file) = AsyncifyFileSystem::new(ctx.root.clone())
                .new_open_options()
                .await
                .read(true)
                .open(alias_check)
                .await
            {
                if already.contains(&name) {
                    break;
                }
                already.insert(name.clone());

                if let Ok(d) = file.read_to_end().await {
                    match serde_yaml::from_slice::<AliasConfig>(&d[..]) {
                        Ok(mut next) => {
                            if next.chroot {
                                chroot = true;
                            }
                            if next.wapm.is_some() {
                                wapm = next.wapm;
                            }
                            if next.base.is_some() {
                                base_dir = next.base;
                            }
                            for (k, v) in next.envs {
                                envs.insert(k, v);
                            }
                            mappings.extend(next.mappings.into_iter());

                            debug!("binary alias '{}' found for {}", next.run, name);
                            name = next.run;
                            alias_loop = true;
                            break;
                        }
                        Err(err) => {
                            debug!("alias file corrupt: /bin/{}.alias - {}", name, err);
                        }
                    }
                }
            }
        }
    }

    // If its a wapm package then check its installed, if not then install it
    if let Some(wapm) = wapm.clone() {
        if let Some(base_dir) = base_dir.clone() {
            wapm_install(ctx, &name, wapm, base_dir).await;
        }
    }       

    // Check if there is a file in the /bin and /usr/bin folder
    let mut file_checks = Vec::new();
    if name.starts_with("/") {
        file_checks.push(name.clone());
    } else if name.starts_with("./") && name.len() > 2 {
        file_checks.push(format!("{}{}", ctx.working_dir, &name[2..]));
    } else {
        file_checks.push(format!("/bin/{}", name));
        file_checks.push(format!("/usr/bin/{}", name));
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
                let mut ret = BinaryPackage::new(d.into());
                if chroot {
                    ret.chroot = true;
                }
                ret.wapm = wapm;
                ret.base_dir = base_dir;
                ret.envs = envs;
                ret.mappings.extend(mappings.into_iter());
                return Some(ret);
            }
        }
    }

    // Resolve some more alias possibilities using fetch commands (with cached results)
    while let Some(next) = ctx.bins.alias(name.as_str()).await {
        if already.contains(&name) {
            break;
        }
        already.insert(name.clone());
        if next.chroot {
            chroot = true;
        }
        if next.wapm.is_some() {
            wapm = next.wapm;
        }
        if next.base.is_some() {
            base_dir = next.base;
        }
        for (k, v) in next.envs {
            envs.insert(k, v);
        }
        name = next.run;
    }

    // Fetch the data asynchronously (from the web site or file system)
    let mut ret = ctx.bins.get(name.as_str(), stdio.stderr.clone()).await;
    if let Some(ret) = ret.as_mut() {
        if chroot {
            ret.chroot = true;
        }
        ret.wapm = wapm;
        ret.base_dir = base_dir;
        ret.envs = envs;
        ret.mappings.extend(mappings.into_iter());
    }
    ret
}

pub async fn wapm_install(ctx: &EvalContext, name: &String, wapm: String, base_dir: String)
{
    let base_dir = Path::new(base_dir.as_str());
    if let Err(wasmer_vfs::FsError::EntityNotFound) = AsyncifyFileSystem::new(ctx.root.clone())
        .read_dir(base_dir)
        .await
    {
        if name.starts_with("wapm ") == true {
            trace!("recursive wapm install blocked.");
            return;
        }

        let (stdin_fd, _) = pipe_in(ReceiverMode::Stream, FdFlag::Stdin(false));
        let mut ctx = ctx.clone();
        ctx.stdio.stdin = stdin_fd;
        ctx.stdio.stdout = ctx.stdio.tty.fd_stdout();
        ctx.stdio.stderr = ctx.stdio.tty.fd_stdout();
        
        let eval_cmd = format!("wapm install {}", wapm);

        let mut process = super::eval(eval_cmd, ctx);
        let result = process.recv().await;
        drop(process);
    } else {
        trace!("confirmed WAPM already installed - {}", wapm);
    }
}