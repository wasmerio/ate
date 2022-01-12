use std::future::Future;
use std::pin::Pin;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::err;
use crate::eval::eval;
use crate::eval::EvalContext;
use crate::eval::EvalStatus;
use crate::eval::ExecResponse;
use crate::fd::FdFlag;
use crate::pipe::*;
use crate::stdio::*;

pub(super) fn wax(
    args: &[String],
    mut ctx: EvalContext,
    stdio: Stdio,
) -> Pin<Box<dyn Future<Output = ExecResponse>>> {
    if args.len() < 2 {
        return Box::pin(async move { ExecResponse::Immediate(ctx, err::ERR_EINVAL) });
    }

    // Strip out the first argument
    let cmd = args[1].clone();
    let args = args.iter().skip(2).map(|a| a.clone()).collect::<Vec<_>>();
    let args = &args[..];

    // Build the script that we will execute
    let mut script = format!("wapm run {} --", cmd);
    for arg in args {
        script.push_str(format!(" {}", arg).as_str());
    }

    // Read the script
    let wax = ctx.bins.wax.clone();
    return Box::pin(async move {
        // If the process is not yet installed then install it
        if wax.lock().unwrap().contains(&cmd) == false {
            let mut tty = ctx.stdio.tty.fd();

            let (stdin_fd, _) = pipe_in(ReceiverMode::Stream, FdFlag::Stdin(false));
            let mut ctx = ctx.clone();
            ctx.stdio.stdin = stdin_fd;
            ctx.stdio.stdout = tty.clone();
            ctx.stdio.stderr = tty.clone();
            ctx.input = format!("wapm install {}", cmd);

            let mut process = eval(ctx);
            let result = process.recv().await;
            drop(process);

            if let Some(EvalStatus::Executed { .. }) = result.map(|a| a.status) {
                wax.lock().unwrap().insert(cmd.clone());
            } else {
                debug!("wax install failed");
                let _ = tty
                    .write("wax: failed to install command\r\n".as_bytes())
                    .await;
                return ExecResponse::OrphanedImmediate(err::ERR_EINVAL);
            }
        }

        // Now actually run the main script
        ctx.stdio = stdio;
        ctx.input = script;

        let mut stdout = ctx.stdio.stdout.clone();
        let mut stderr = ctx.stdio.stderr.clone();

        let mut process = eval(ctx);
        let result = process.recv().await;
        drop(process);

        let result = match result {
            Some(a) => a,
            None => {
                debug!("wax recv error");
                let _ = stderr
                    .write(format!("wax: command failed\r\n").as_bytes())
                    .await;
                return ExecResponse::OrphanedImmediate(err::ERR_EINTR);
            }
        };

        let ctx = result.ctx;
        match result.status {
            EvalStatus::Executed { code, show_result } => {
                debug!("wax executed (code={})", code);
                if code != 0 && show_result {
                    let mut chars = String::new();
                    chars += err::exit_code_to_message(code);
                    chars += "\r\n";
                    let _ = stdout.write(chars.as_bytes()).await;
                }
                ExecResponse::Immediate(ctx, code)
            }
            EvalStatus::InternalError => {
                debug!("wax internal error");
                let _ = stderr.write("wax: internal error\r\n".as_bytes()).await;
                ExecResponse::Immediate(ctx, err::ERR_EINTR)
            }
            EvalStatus::MoreInput => {
                debug!("wax more input");
                let _ = stderr.write("wax: incomplete command\r\n".as_bytes()).await;
                ExecResponse::Immediate(ctx, err::ERR_EINVAL)
            }
            EvalStatus::Invalid => {
                debug!("wax invalid");
                let _ = stderr.write("wax: invalid command\r\n".as_bytes()).await;
                ExecResponse::Immediate(ctx, err::ERR_EINVAL)
            }
        }
    });
}
