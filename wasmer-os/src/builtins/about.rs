use std::future::Future;
use std::pin::Pin;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::eval::EvalContext;
use crate::eval::ExecResponse;
use crate::stdio::*;
use crate::tty::Tty;

pub(super) fn about(
    args: &[String],
    ctx: EvalContext,
    mut stdio: Stdio,
) -> Pin<Box<dyn Future<Output = ExecResponse> + Send>> {
    let txt = if args.len() <= 1 {
        Tty::ABOUT
    } else {
        match args[1].as_str() {
            "deploy" => Tty::ABOUT_DEPLOY,
            "wasmer" => Tty::ABOUT_WASMER,
            _ => Tty::ABOUT,
        }
    };

    Box::pin(async move {
        let _ = stdio.stdout.write(txt.as_bytes()).await;
        let _ = stdio.stdout.write("\r\n".as_bytes()).await;
        ExecResponse::Immediate(ctx, 0)
    })
}
