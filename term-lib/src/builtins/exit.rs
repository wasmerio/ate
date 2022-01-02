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
    let abi = ctx.abi.clone();
    Box::pin(async move {
        abi.exit().await;
        ExecResponse::Immediate(0).into()
    })
}
