
#[allow(unused_imports, dead_code)]
use tracing::{info, error, debug, trace, warn};
use std::pin::Pin;
use std::future::Future;

use crate::stdio::*;
use crate::tty::Tty;
use crate::eval::EvalContext;
use crate::eval::ExecResponse;

pub(super) fn about(args: &[String], _ctx: &mut EvalContext, mut stdio: Stdio) -> Pin<Box<dyn Future<Output = Result<ExecResponse, i32>>>> {
    let txt = if args.len() <= 1 {
        Tty::ABOUT
    } else {
        match args[1].as_str() {
            "tokera" => Tty::ABOUT_TOKERA,
            "wasmer" => Tty::ABOUT_WASMER,
            _ => Tty::ABOUT
        }
    };

    Box::pin(async move {
        let _ = stdio.stdout.write(txt.as_bytes()).await;
        let _ = stdio.stdout.write("\r\n".as_bytes()).await;
        Ok(ExecResponse::Immediate(0))
    })
}