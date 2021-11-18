use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use wasmer_vfs::FileSystem;

use crate::eval::EvalContext;
use crate::eval::ExecResponse;
use crate::stdio::*;

pub(super) fn cd(
    args: &[String],
    ctx: &mut EvalContext,
    mut stdio: Stdio,
) -> Pin<Box<dyn Future<Output = Result<ExecResponse, i32>>>> {
    if args.len() > 2 {
        return Box::pin(async move {
            let _ = stdio
                .stderr
                .write(format!("cd: too many arguments\r\n").as_bytes())
                .await;
            Ok(ExecResponse::Immediate(0))
        });
    }

    let mut print_path = false;
    let mut dir = if args.len() == 1 {
        home(ctx)
    } else if args[1] == "-" {
        if let Some(v) = ctx.env.get("OLDPWD") {
            print_path = true;
            v
        } else {
            return Box::pin(async move {
                let _ = stdio
                    .stderr
                    .write(format!("cd: -: OLDPWD not set\r\n").as_bytes())
                    .await;
                Ok(ExecResponse::Immediate(0))
            });
        }
    } else {
        let mut dir = args[1].clone();
        if dir.starts_with("/") == false {
            dir.insert_str(0, current(ctx).as_str());
        }

        dir = canonicalize(dir.as_str());
        if ctx.root.read_dir(Path::new(dir.as_str())).is_err() {
            return Box::pin(async move {
                let _ = stdio
                    .stderr
                    .write(format!("cd: {}: No such directory\r\n", dir).as_bytes())
                    .await;
                Ok(ExecResponse::Immediate(0))
            });
        }
        dir
    };

    if dir.ends_with("/") == false {
        dir += "/";
    }

    ctx.env.set_var("OLDPWD", current(ctx));
    set_current(ctx, dir.as_str());
    ctx.env.set_var("PWD", dir.clone());

    Box::pin(async move {
        if print_path {
            let _ = stdio.stdout.write(format!("{}\r\n", dir).as_bytes()).await;
        }
        Ok(ExecResponse::Immediate(0))
    })
}

fn canonicalize(path: &str) -> String {
    let mut ret = String::with_capacity(path.len());

    let mut comps = Vec::new();
    for comp in path.split("/") {
        if comp.len() <= 0 {
            continue;
        }
        if comp == "." {
            continue;
        };
        if comp == ".." {
            if comps.len() > 0 {
                comps.remove(comps.len() - 1);
            }
            continue;
        }
        comps.push(comp);
    }

    ret += "/";
    for comp in comps {
        if ret.ends_with("/") == false {
            ret += "/";
        }
        ret += comp;
    }
    ret
}

fn home(ctx: &EvalContext) -> String {
    ctx.env.get("HOME").unwrap_or_else(|| String::from("/"))
}

fn current(ctx: &EvalContext) -> String {
    let console = ctx.console.lock().unwrap();
    console.path.clone()
}

fn set_current(ctx: &EvalContext, path: &str) {
    let mut console = ctx.console.lock().unwrap();
    console.path = path.to_string();
}
