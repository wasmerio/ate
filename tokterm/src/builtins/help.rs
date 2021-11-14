use std::pin::Pin;
use std::future::Future;

use crate::stdio::*;
use crate::tty::Tty;
use crate::eval::EvalContext;
use crate::eval::ExecResponse;

pub(super) fn help(_args: &[String], _ctx: &mut EvalContext, mut stdio: Stdio) -> Pin<Box<dyn Future<Output = Result<ExecResponse, i32>>>> {
    Box::pin(async move {
        let _ = stdio.stdout.write(Tty::HELP.as_bytes()).await;
        Ok(ExecResponse::Immediate(0))
    })
}