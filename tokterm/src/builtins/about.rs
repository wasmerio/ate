
#[allow(unused_imports, dead_code)]
use tracing::{info, error, debug, trace, warn};
use std::pin::Pin;
use std::future::Future;

use crate::stdio::*;
use crate::tty::Tty;
use crate::eval::EvalContext;

pub(super) fn about(args: &[String], _ctx: &mut EvalContext, mut stdio: Stdio) -> Pin<Box<dyn Future<Output = i32>>> {
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
        0
    })
}