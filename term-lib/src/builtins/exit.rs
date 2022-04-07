use std::future::Future;
use std::pin::Pin;

use crate::eval::EvalContext;
use crate::eval::ExecResponse;
use crate::stdio::*;

pub(super) fn exit(
    _args: &[String],
    ctx: EvalContext,
    _stdio: Stdio,
) -> Pin<Box<dyn Future<Output = ExecResponse> + Send>> {
    Box::pin(async move {
        ctx.abi.exit().await;
        ExecResponse::Immediate(ctx, 0)
    })
}
