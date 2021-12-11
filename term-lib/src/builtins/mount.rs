use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::backend::process::StdioMode;

use super::CommandResult;
use crate::bus::ProcessExecFactory;
use crate::bus::SubProcessFactory;
use crate::eval::EvalContext;
use crate::eval::ExecResponse;
use crate::fs::FuseFileSystem;
use crate::fs::MountPoint;
use crate::stdio::*;
use crate::tty::*;

pub(super) fn mount(
    args: &[String],
    ctx: &mut EvalContext,
    mut stdio: Stdio,
) -> Pin<Box<dyn Future<Output = CommandResult>>> {
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
                let _ = stdio
                    .stderr
                    .write(format!("mount: too many arguments\r\n").as_bytes())
                    .await;
                ExecResponse::Immediate(0).into()
            });
        }
        _ => {
            return Box::pin(async move {
                let _ = stdio
                    .stderr
                    .write(Tty::MOUNT_USAGE.to_string().as_bytes())
                    .await;
                ExecResponse::Immediate(0).into()
            });
        }
    }

    let factory = ProcessExecFactory::new(
        ctx.reactor.clone(),
        ctx.exec_factory.clone(),
        stdio.stdin.downgrade(),
        stdio.stdout.downgrade(),
        stdio.stderr.downgrade(),
        stdio.log.downgrade(),
    );

    let mut ctx = ctx.clone();
    return Box::pin(async move {
        let _ = stdio
            .stdout
            .write(format!("Mounting {}@{} at {}\r\n", target, wapm, mountpoint).as_bytes())
            .await;

        let factory = SubProcessFactory::new(factory);
        let sub_process = match factory.get_or_create(wapm.as_str(), StdioMode::Log).await {
            Ok(a) => a,
            Err(_) => {
                let _ = stdio
                    .stderr
                    .write(format!("mount: wapm program not found\r\n").as_bytes())
                    .await;
                return ExecResponse::Immediate(0).into();
            }
        };

        let _ = stdio
            .stdout
            .write(format!("Waiting for poll\r\n").as_bytes())
            .await;

        let fs = FuseFileSystem::new(sub_process, target.as_str());

        let _ = stdio
            .stdout
            .write(format!("Successfully mounted\r\n").as_bytes())
            .await;

        let mut ret: CommandResult = ExecResponse::Immediate(0).into();
        ctx.new_mounts.push(MountPoint {
            name: wapm,
            path: mountpoint,
            fs: Arc::new(Box::new(fs)),
        });
        ret.ctx = Some(ctx);
        ret
    });
}
