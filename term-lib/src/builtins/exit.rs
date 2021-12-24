use std::future::Future;
use std::pin::Pin;

use super::CommandResult;
use crate::eval::EvalContext;
use crate::eval::ExecResponse;
use crate::stdio::*;

pub(super) fn exit(
    _args: &[String],
    ctx: &mut EvalContext,
    _stdio: Stdio,
) -> Pin<Box<dyn Future<Output = CommandResult>>> {
    ctx.system.exit();
    Box::pin(async move { ExecResponse::Immediate(0).into() })
}
