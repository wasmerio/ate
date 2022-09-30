use std::future::Future;
use std::pin::Pin;

use crate::eval::EvalContext;
use crate::eval::ExecResponse;
use crate::stdio::*;
use crate::tty::*;

#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

#[allow(dead_code, unused)]
pub(super) fn call(
    args: &[String],
    mut ctx: EvalContext,
    mut stdio: Stdio,
) -> Pin<Box<dyn Future<Output = ExecResponse> + Send>> {
    let instance: String;
    let wapm: String;
    let topic: String;
    let access_token: Option<String>;
    match args.len() {
        4 => {
            instance = args[1].clone();
            wapm = args[2].clone();
            topic = args[3].clone();
            access_token = None;
        }
        5 => {
            instance = args[1].clone();
            wapm = args[2].clone();
            topic = args[3].clone();
            access_token = Some(args[4].clone());
        }
        a if a > 5 => {
            return Box::pin(async move {
                print(format!("call: too many arguments\r\n"), &mut stdio, true).await;
                ExecResponse::Immediate(ctx, 0)
            });
        }
        _ => {
            return Box::pin(async move {
                print(Tty::CALL_USAGE.to_string(), &mut stdio, false).await;
                ExecResponse::Immediate(ctx, 0)
            });
        }
    }

    return Box::pin(async move {
        print(format!("call: not yet implemented\r\n"), &mut stdio, true).await;
        ExecResponse::Immediate(ctx, 0)
    });
}

async fn print(msg: String, stdio: &mut Stdio, is_err: bool) {
    if is_err {
        error!("{}", msg);
        let _ = stdio.stdout.write(msg.as_bytes()).await;
        let _ = stdio.stdout.flush_async().await;
    } else {
        let _ = stdio.stderr.write(msg.as_bytes()).await;
        let _ = stdio.stderr.flush_async().await;
    }
}
