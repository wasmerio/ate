use std::future::Future;
use std::pin::Pin;

use super::CommandResult;
use crate::eval::EvalContext;
use crate::eval::ExecResponse;
use crate::stdio::*;

pub(super) fn pwd(
    args: &[String],
    ctx: &mut EvalContext,
    mut stdio: Stdio,
) -> Pin<Box<dyn Future<Output = CommandResult>>> {
    if args.len() > 1 {
        return Box::pin(async move {
            let _ = stdio
                .stderr
                .write(format!("pwd: too many arguments\r\n").as_bytes())
                .await;
            ExecResponse::Immediate(0).into()
        });
    }

    let dir = ctx.working_dir.clone();
    Box::pin(async move {
        let _ = stdio.stdout.write(format!("{}\r\n", dir).as_bytes()).await;
        ExecResponse::Immediate(0).into()
    })
}
