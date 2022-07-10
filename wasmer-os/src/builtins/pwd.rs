use std::future::Future;
use std::pin::Pin;

use crate::eval::EvalContext;
use crate::eval::ExecResponse;
use crate::stdio::*;

pub(super) fn pwd(
    args: &[String],
    ctx: EvalContext,
    mut stdio: Stdio,
) -> Pin<Box<dyn Future<Output = ExecResponse> + Send>> {
    if args.len() > 1 {
        return Box::pin(async move {
            let _ = stdio
                .stderr
                .write(format!("pwd: too many arguments\r\n").as_bytes())
                .await;
            ExecResponse::Immediate(ctx, 0)
        });
    }
    Box::pin(async move {
        let _ = stdio
            .stdout
            .write(format!("{}\r\n", ctx.working_dir).as_bytes())
            .await;
        ExecResponse::Immediate(ctx, 0)
    })
}
