use std::future::Future;
use std::pin::Pin;

use crate::eval::EvalContext;
use crate::eval::ExecResponse;
use crate::stdio::*;

pub(super) fn unset(
    args: &[String],
    mut ctx: EvalContext,
    _stdio: Stdio,
) -> Pin<Box<dyn Future<Output = ExecResponse>>> {
    for arg in &args[1..] {
        ctx.env.unset(arg.as_str());
    }
    Box::pin(async move { ExecResponse::Immediate(ctx, 0) })
}
