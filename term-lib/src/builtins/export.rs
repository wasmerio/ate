use std::future::Future;
use std::pin::Pin;

use crate::eval::EvalContext;
use crate::eval::ExecResponse;
use crate::stdio::*;

pub(super) fn export(
    args: &[String],
    mut ctx: EvalContext,
    stdio: Stdio,
) -> Pin<Box<dyn Future<Output = ExecResponse> + Send>> {
    if args.len() <= 1 || args[1] == "-p" {
        let output = ctx
            .env
            .iter()
            .filter(|(_, v)| v.export)
            .map(|(k, v)| {
                if let Some(veq) = &v.var_eq {
                    format!("export {}", veq)
                } else {
                    format!("export {}", k)
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
            ctx.env.export(key.as_str());
            ctx.env.set_var(&key, value);
        } else {
            ctx.env.export(arg.as_str())
        }
    }

    Box::pin(async move { ExecResponse::Immediate(ctx, 0) })
}
