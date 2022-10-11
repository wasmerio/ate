use std::future::Future;
use std::pin::Pin;

use crate::eval::EvalContext;
use crate::eval::ExecResponse;
use crate::stdio::*;

pub(super) fn readonly(
    args: &[String],
    mut ctx: EvalContext,
    stdio: Stdio,
) -> Pin<Box<dyn Future<Output = ExecResponse> + Send>> {
    if args.len() == 1 || args[1] == "-p" {
        let output = ctx
            .env
            .iter()
            .filter(|(_, v)| v.readonly)
            .map(|(k, v)| {
                if let Some(veq) = &v.var_eq {
                    format!("readonly {}", veq)
                } else {
                    format!("readonly {}", k)
                }
            })
            .collect::<Vec<_>>();
        return Box::pin(async move {
            for output in output {
                let _ = stdio.println(output).await;
            }
            ExecResponse::Immediate(ctx, 0)
        });
    }

    for arg in &args[1..] {
        if arg.contains('=') {
            let (key, value) = ctx.env.parse_key_value(arg);
            ctx.env.readonly(key.as_str());
            ctx.env.set_var(&key, value);
        } else {
            ctx.env.readonly(arg.as_str())
        }
    }

    Box::pin(async move { ExecResponse::Immediate(ctx, 0) })
}
