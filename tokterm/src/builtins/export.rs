use std::pin::Pin;
use std::future::Future;

use crate::stdio::*;
use crate::eval::EvalContext;
use crate::eval::ExecResponse;

pub(super) fn export(args: &[String], ctx: &mut EvalContext, stdio: Stdio) -> Pin<Box<dyn Future<Output = Result<ExecResponse, i32>>>> {
    if args.len() <= 1 || args[1] == "-p" {
        let output = ctx.env
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
                let _ = stdio.println(format_args!("{}", output)).await;
            }
            Ok(ExecResponse::Immediate(0))
        });
    }

    for arg in &args[1..] {
        if arg.contains('=') {
            let key = ctx.env.parse_key(arg);
            ctx.env.export(key.as_str());
            ctx.env.set_vareq_with_key(key, arg.clone());
        } else {
            ctx.env.export(arg.as_str())
        }
    }

    Box::pin(async move {
        Ok(ExecResponse::Immediate(0))
    })
}