use std::future::Future;
use std::pin::Pin;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::eval::EvalContext;
use crate::eval::ExecResponse;
use crate::stdio::*;
use crate::tty::Tty;

pub(super) fn builtin(
    _args: &[String],
    ctx: EvalContext,
    mut stdio: Stdio,
) -> Pin<Box<dyn Future<Output = ExecResponse> + Send>> {
    Box::pin(async move {
        let _ = stdio.stdout.write(Tty::BUILTIN.as_bytes()).await;
        ExecResponse::Immediate(ctx, 0)
    })
}
