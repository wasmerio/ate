use std::pin::Pin;
use std::future::Future;

use crate::stdio::*;
use crate::eval::EvalContext;

pub(super) fn unset(args: &[String], ctx: &mut EvalContext, _stdio: Stdio) -> Pin<Box<dyn Future<Output = i32>>> {
    for arg in &args[1..] {
        ctx.env.unset(arg.as_str());
    }
    Box::pin(async move {
        0
    })
}