use std::pin::Pin;
use std::future::Future;

use crate::stdio::*;
use crate::eval::EvalContext;

pub(super) fn readonly(args: &[String], ctx: &mut EvalContext, stdio: Stdio) -> Pin<Box<dyn Future<Output = i32>>> {
    if args.len() == 1 || args[1] == "-p" {
        let output = ctx.env
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
                let _ = stdio.println(format_args!("{}", output)).await;
            }
            0
        });
    }

    for arg in &args[1..] {
        if arg.contains('=') {
            let key = ctx.env.parse_key(arg);
            ctx.env.readonly(key.as_str());
            ctx.env.set_vareq_with_key(key, arg.clone());
        } else {
            ctx.env.readonly(arg.as_str())
        }
    }

    Box::pin(async move {
        0
    })
}