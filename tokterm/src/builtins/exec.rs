#[allow(unused_imports, dead_code)]
use tracing::{info, error, debug, trace, warn};
use std::pin::Pin;
use std::future::Future;
use wasmer_wasi::vfs::FileSystem;
use std::path::Path;

use crate::err;
use crate::stdio::*;
use crate::eval::EvalContext;
use crate::eval::ExecResponse;
use crate::eval::EvalPlan;
use crate::eval::eval;

pub(super) fn exec(args: &[String], ctx: &mut EvalContext, mut stdio: Stdio) -> Pin<Box<dyn Future<Output = Result<ExecResponse, i32>>>> {
    if args.len() != 2 {
        return Box::pin(async move {
            Ok(ExecResponse::Immediate(err::ERR_EINVAL))
        });
    }

    // Read the script
    let script = args[1].clone();
    let script = ctx.stdio.root.new_open_options()
        .read(true)
        .open(&Path::new(&script));
    let mut script = match script {
        Ok(a) => a,
        Err(_) => {
            return Box::pin(async move {
                let _ = stdio.stderr.write(format!("exec: script not found\r\n").as_bytes()).await;
                Ok(ExecResponse::Immediate(1))
            });
        }
    };
    let script = {
        let mut s = String::new();
        if let Err(_) = script.read_to_string(&mut s) {
            return Box::pin(async move {
                let _ = stdio.stderr.write(format!("exec: script not readable\r\n").as_bytes()).await;
                Ok(ExecResponse::Immediate(err::ERR_ENOENT))
            });
        }
        s
    };

    let mut ctx = ctx.clone();
    ctx.input = script;

    // Now run the evaluation again but using this new scripts
    let state = ctx.console.clone();
    Box::pin(async move {
        let mut stdout = ctx.stdio.stdout.clone();
        let mut stderr = ctx.stdio.stderr.clone();
        match eval(ctx).await.await {
            Ok(EvalPlan::Executed { code, ctx, show_result }) => {
                debug!("exec executed (code={})", code);
                let should_line_feed = {
                    let mut state = state.lock().unwrap();
                    state.last_return = code;
                    state.env = ctx.env;
                    state.unfinished_line
                };
                if code != 0 && show_result {
                    let mut chars = String::new();
                    chars += err::exit_code_to_message(code);
                    chars += "\r\n";
                    let _ = stdout.write(chars.as_bytes()).await;
                } else if should_line_feed {
                    let _ = stdout.write("\r\n".as_bytes()).await;
                }
                Ok(ExecResponse::Immediate(code))
            },
            Ok(EvalPlan::InternalError) => {
                debug!("eval internal error");
                let _ = stderr.write("exec: internal error\r\n".as_bytes()).await;
                Ok(ExecResponse::Immediate(err::ERR_EINTR))
            }
            Ok(EvalPlan::MoreInput) => {
                debug!("eval more input");
                let _ = stderr.write("exec: incomplete command\r\n".as_bytes()).await;
                Ok(ExecResponse::Immediate(err::ERR_EINVAL))
            }
            Ok(EvalPlan::Invalid) => {
                debug!("eval invalid");
                let _ = stderr.write("exec: invalid command\r\n".as_bytes()).await;
                Ok(ExecResponse::Immediate(err::ERR_EINVAL))
            },
            Err(err) => {
                debug!("eval recv error (err={})", err);
                let _ = stderr.write(format!("exec: command failed - {} \r\n", err).as_bytes()).await;
                Ok(ExecResponse::Immediate(err::ERR_EINTR))
            }
        }
    })
}