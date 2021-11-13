use std::pin::Pin;
use std::future::Future;

use crate::stdio::*;
use crate::tty::Tty;
use crate::eval::EvalContext;

pub(super) fn about_tokera(_args: &[String], _ctx: &mut EvalContext, mut stdio: Stdio) -> Pin<Box<dyn Future<Output = i32>>> {
    Box::pin(async move {
        let _ = stdio.stdout.write(Tty::ABOUT_TOKERA.as_bytes()).await;
        0
    })
}

pub(super) fn about_wasmer(_args: &[String], _ctx: &mut EvalContext, mut stdio: Stdio) -> Pin<Box<dyn Future<Output = i32>>> {
    Box::pin(async move {
        let _ = stdio.stdout.write(Tty::ABOUT_WASMER.as_bytes()).await;
        0
    })
}