use std::future::Future;
use std::path::Path;
use std::pin::Pin;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use super::CommandResult;
use crate::err;
use crate::eval::eval;
use crate::eval::EvalContext;
use crate::eval::EvalPlan;
use crate::eval::ExecResponse;
use crate::fs::*;
use crate::stdio::*;

pub(super) fn source(
    args: &[String],
    ctx: &mut EvalContext,
    mut stdio: Stdio,
) -> Pin<Box<dyn Future<Output = CommandResult>>> {
    if args.len() != 2 {
        return Box::pin(async move { ExecResponse::Immediate(err::ERR_EINVAL).into() });
    }

    // Read the script
    let script = args[1].clone();
    let mut ctx = ctx.clone();
    return Box::pin(async move {
        let script = AsyncifyFileSystem::new(ctx.root.clone())
            .new_open_options()
            .await
            .read(true)
            .open(&Path::new(&script))
            .await;
        let mut script = match script {
            Ok(a) => a,
            Err(_) => {
                let _ = stdio
                    .stderr
                    .write(format!("exec: script not found\r\n").as_bytes())
                    .await;
                return ExecResponse::Immediate(1).into();
            }
        };
        let script = {
            match script.read_to_string().await {
                Ok(s) => s,
                Err(_err) => {
                    let _ = stdio
                        .stderr
                        .write(format!("exec: script not readable\r\n").as_bytes())
                        .await;
                    return ExecResponse::Immediate(err::ERR_ENOENT).into();
                }
            }
        };

        ctx.stdio = stdio;
        ctx.input = script;

        let mut stdout = ctx.stdio.stdout.clone();
        let mut stderr = ctx.stdio.stderr.clone();

        let mut process = eval(ctx);
        let result = process.recv().await;
        drop(process);

        match result {
            Some(EvalPlan::Executed {
                code,
                ctx,
                show_result,
            }) => {
                debug!("exec executed (code={})", code);
                if code != 0 && show_result {
                    let mut chars = String::new();
                    chars += err::exit_code_to_message(code);
                    chars += "\r\n";
                    let _ = stdout.write(chars.as_bytes()).await;
                }
                let mut ret: CommandResult = ExecResponse::Immediate(code).into();
                ret.ctx = Some(ctx);
                ret
            }
            Some(EvalPlan::InternalError) => {
                debug!("eval internal error");
                let _ = stderr.write("exec: internal error\r\n".as_bytes()).await;
                ExecResponse::Immediate(err::ERR_EINTR).into()
            }
            Some(EvalPlan::MoreInput) => {
                debug!("eval more input");
                let _ = stderr
                    .write("exec: incomplete command\r\n".as_bytes())
                    .await;
                ExecResponse::Immediate(err::ERR_EINVAL).into()
            }
            Some(EvalPlan::Invalid) => {
                debug!("eval invalid");
                let _ = stderr.write("exec: invalid command\r\n".as_bytes()).await;
                ExecResponse::Immediate(err::ERR_EINVAL).into()
            }
            None => {
                debug!("eval recv error");
                let _ = stderr
                    .write(format!("exec: command failed\r\n").as_bytes())
                    .await;
                ExecResponse::Immediate(err::ERR_EINTR).into()
            }
        }
    });
}
