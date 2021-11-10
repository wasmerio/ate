use std::env as stdenv;
use std::path::Path;
use std::pin::Pin;
use std::future::Future;

use crate::stdio::*;
use crate::eval::EvalContext;

pub(super) fn cd(args: &[String], ctx: &mut EvalContext, stdio: Stdio) -> Pin<Box<dyn Future<Output = i32>>> {
    if args.len() > 2 {
        return Box::pin(async move {
            let _ = stdio.eprintln(format_args!("too many arguments")).await;
            1
        });
    }

    let mut print_path = false;
    let dir = if args.len() == 1 {
        ctx.env.get("HOME").unwrap_or_else(|| String::from("/"))
    } else if args[1] == "-" {
        if let Some(v) = ctx.env.get("OLDPWD") {
            print_path = true;
            v
        } else {
            return Box::pin(async move {
                let _ = stdio.eprintln(format_args!("cd: -: OLDPWD not set")).await;
                1
            });
        }
    } else {
        args[1].clone()
    };

    let path = Path::new(&dir);
    let old = stdenv::current_dir();

    match stdenv::set_current_dir(path) {
        Ok(_) => {
            if let Ok(oldpwd) = old {
                ctx.env.set_var("OLDPWD", oldpwd.to_string_lossy().to_string());
            }
            let path = path.to_string_lossy().to_string();
            Box::pin(async move {
                if print_path {
                    let _ = stdio.println(format_args!("{}", path)).await;
                }
                0
            })
        }
        Err(e) => {
            let path = path.display().to_string();
            Box::pin(async move {
                let _ = stdio.eprintln(format_args!(
                    "cd: {}: {}",
                    path,
                    e.to_string()
                )).await;
                1
            })
        }
    }
}