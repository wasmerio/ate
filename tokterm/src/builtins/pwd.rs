use std::future::Future;
use std::pin::Pin;

use crate::eval::EvalContext;
use crate::eval::ExecResponse;
use crate::stdio::*;

pub(super) fn pwd(
    args: &[String],
    ctx: &mut EvalContext,
    mut stdio: Stdio,
) -> Pin<Box<dyn Future<Output = Result<ExecResponse, i32>>>> {
    if args.len() > 1 {
        return Box::pin(async move {
            let _ = stdio
                .stderr
                .write(format!("pwd: too many arguments\r\n").as_bytes())
                .await;
            Ok(ExecResponse::Immediate(0))
        });
    }

    let dir = ctx.path.clone();    
    Box::pin(async move {
        let _ = stdio.stdout.write(format!("{}\r\n", dir).as_bytes()).await;
        Ok(ExecResponse::Immediate(0))
    })
}
