use std::future::Future;
use std::pin::Pin;

use super::CommandResult;
use crate::eval::EvalContext;
use crate::eval::ExecResponse;
use crate::stdio::*;

pub(super) fn unset(
    args: &[String],
    ctx: &mut EvalContext,
    _stdio: Stdio,
) -> Pin<Box<dyn Future<Output = CommandResult>>> {
    for arg in &args[1..] {
        ctx.env.unset(arg.as_str());
    }
    Box::pin(async move { ExecResponse::Immediate(0).into() })
}
