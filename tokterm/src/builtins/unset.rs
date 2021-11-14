use std::pin::Pin;
use std::future::Future;

use crate::stdio::*;
use crate::eval::EvalContext;
use crate::eval::ExecResponse;

pub(super) fn unset(args: &[String], ctx: &mut EvalContext, _stdio: Stdio) -> Pin<Box<dyn Future<Output = Result<ExecResponse, i32>>>> {
    for arg in &args[1..] {
        ctx.env.unset(arg.as_str());
    }
    Box::pin(async move {
        Ok(ExecResponse::Immediate(0))
    })
}