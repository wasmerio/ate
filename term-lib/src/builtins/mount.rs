use std::future::Future;
use std::path::Path;
use std::pin::Pin;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus_process::prelude::StdioMode;
use wasmer_vfs::FileSystem;

use crate::bus::ProcessExecFactory;
use crate::bus::SubProcessFactory;
use crate::eval::EvalContext;
use crate::eval::ExecResponse;
use crate::fs::FuseFileSystem;
use crate::stdio::*;
use crate::tty::*;

pub(super) fn mount(
    args: &[String],
    mut ctx: EvalContext,
    mut stdio: Stdio,
) -> Pin<Box<dyn Future<Output = ExecResponse>>> {
    let wapm: String;
    let mountpoint: String;
    let target: String;
    match args.len() {
        3 => {
            wapm = "tok".to_string();
            mountpoint = args[1].clone();
            target = args[2].clone();
        }
        4 => {
            wapm = args[1].clone();
            mountpoint = args[2].clone();
            target = args[3].clone();
        }
        a if a > 4 => {
            return Box::pin(async move {
                print(format!("mount: too many arguments\r\n"), &mut stdio, true).await;
                ExecResponse::Immediate(ctx, 0)
            });
        }
        _ => {
            let mounts = ctx.root.mounts.clone();
            return Box::pin(async move {
                print(Tty::MOUNT_USAGE.to_string(), &mut stdio, false).await;

                // Display the existing mounts
                if mounts.len() > 0 {
                    print("\r\nCurrent Mounts:\r\n".to_string(), &mut stdio, false).await;
                    for mount in mounts.iter() {
                        print(format!("{}\twith\t{}\r\n", mount.path, mount.name), &mut stdio, false).await;
                    }
                }

                ExecResponse::Immediate(ctx, 0)
            });
        }
    }

    let factory = ProcessExecFactory::new(
        ctx.abi.clone(),
        ctx.reactor.clone(),
        ctx.compiler,
        ctx.exec_factory.clone(),
        stdio.stdin.downgrade(),
        stdio.stdout.downgrade(),
        stdio.stderr.downgrade(),
        stdio.log.downgrade(),
        ctx.clone(),
    );

    return Box::pin(async move {
        let path_mountpoint = Path::new(mountpoint.as_str());
        if let Err(err) = ctx.root.read_dir(path_mountpoint) {
            print(format!("mount: the mountpoint is invalid: {}\r\n", err), &mut stdio, true).await;
            return ExecResponse::Immediate(ctx, 1);
        }

        print(format!("Mounting {}@{} at {}\r\n", target, wapm, mountpoint), &mut stdio, false).await;

        let factory = SubProcessFactory::new(factory);
        let sub_process = match factory
            .get_or_create(wapm.as_str(), StdioMode::Inherit, StdioMode::Log)
            .await
        {
            Ok(a) => a,
            Err(_) => {
                print(format!("mount: wapm program not found\r\n"), &mut stdio, true).await;
                return ExecResponse::Immediate(ctx, 1);
            }
        };

        print(format!("Waiting for poll\r\n"), &mut stdio, false).await;

        let mut ready = false;
        tokio::select! {
            _ = ctx.system.sleep(5000) => { },
            r = sub_process.main.async_wait_for_poll() => {
                ready = r;
            }
        }
        if ready == false {
            print(format!("mount: wapm program failed to poll\r\n"), &mut stdio, true).await;
            return ExecResponse::Immediate(ctx, 1);
        }

        print(format!("Executing the mount\r\n"), &mut stdio, false).await;

        let fs = match FuseFileSystem::new(sub_process, target.as_str(), stdio.clone()).await {
            Ok(a) => a,
            Err(err) => {
                print(format!("mount: mount call failed ({})\r\n", err), &mut stdio, true).await;
                return ExecResponse::Immediate(ctx, 1);
            }
        };
        let _ = stdio.stdout.flush_async().await;

        print(format!("\rSuccessfully mounted\r\n"), &mut stdio, false).await;

        ctx.root.mount(
            format!("{}({})", wapm, target).as_str(),
            mountpoint.as_str(),
            false,
            Box::new(fs));

        ExecResponse::Immediate(ctx, 0)
    });
}

async fn print(msg: String, stdio: &mut Stdio, is_err: bool) {
    if is_err {
        error!("{}", msg);
        let _ = stdio.stdout.write(msg.as_bytes()).await;
        let _ = stdio.stdout.flush_async().await;
    } else {
        let _ = stdio.stderr.write(msg.as_bytes()).await;
        let _ = stdio.stderr.flush_async().await;
    }
}
