use std::future::Future;
use std::pin::Pin;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::eval::EvalContext;
use crate::eval::ExecResponse;
use crate::stdio::*;
use crate::tty::*;

pub(super) fn umount(
    args: &[String],
    mut ctx: EvalContext,
    mut stdio: Stdio,
) -> Pin<Box<dyn Future<Output = ExecResponse> + Send>> {
    let mountpoint: String;
    match args.len() {
        2 => {
            mountpoint = args[1].clone();
        }
        a if a > 2 => {
            return Box::pin(async move {
                print(format!("umount: too many arguments\r\n"), &mut stdio, true).await;
                ExecResponse::Immediate(ctx, 0)
            });
        }
        _ => {
            return Box::pin(async move {
                print(Tty::UMOUNT_USAGE.to_string(), &mut stdio, true).await;
                ExecResponse::Immediate(ctx, 0)
            });
        }
    }

    return Box::pin(async move {
        ctx.root.unmount(mountpoint.as_str());
        print(format!("Unmounted {}\r\n", mountpoint), &mut stdio, false).await;
        ExecResponse::Immediate(ctx, 0)
    });
}

async fn print(msg: String, stdio: &mut Stdio, is_err: bool) {
    if is_err {
        error!("{}", msg);
        let _ = stdio.stdout.write(msg.as_bytes()).await;
        let _ = stdio.stdout.flush_async().await;
    } else {
        info!("{}", msg);
        let _ = stdio.stderr.write(msg.as_bytes()).await;
        let _ = stdio.stderr.flush_async().await;
    }
}
