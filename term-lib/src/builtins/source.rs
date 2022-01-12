use std::collections::VecDeque;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::err;
use crate::eval::eval;
use crate::eval::EvalContext;
use crate::eval::EvalStatus;
use crate::eval::ExecResponse;
use crate::fs::*;
use crate::stdio::*;

pub(super) fn source(
    args: &[String],
    mut ctx: EvalContext,
    mut stdio: Stdio,
) -> Pin<Box<dyn Future<Output = ExecResponse>>> {
    if args.len() != 2 {
        return Box::pin(async move { ExecResponse::Immediate(ctx, err::ERR_EINVAL) });
    }

    // Read the script
    let script = args[1].clone();
    return Box::pin(async move {
        let script = AsyncifyFileSystem::new(ctx.root.clone())
            .new_open_options()
            .await
            .read(true)
            .open(&Path::new(&script))
            .await;
        let mut script = match script {
            Ok(a) => a,
            Err(_) => {
                let _ = stdio
                    .stderr
                    .write(format!("exec: script not found\r\n").as_bytes())
                    .await;
                return ExecResponse::Immediate(ctx, 1);
            }
        };
        let script = {
            match script.read_to_string().await {
                Ok(s) => s,
                Err(_err) => {
                    let _ = stdio
                        .stderr
                        .write(format!("exec: script not readable\r\n").as_bytes())
                        .await;
                    return ExecResponse::Immediate(ctx, err::ERR_ENOENT);
                }
            }
        };

        let script = process_script(script, &ctx);

        ctx.stdio = stdio;
        ctx.input = script;

        let mut stdout = ctx.stdio.stdout.clone();
        let mut stderr = ctx.stdio.stderr.clone();

        let mut process = eval(ctx);
        let result = process.recv().await;
        drop(process);

        let result = match result {
            Some(a) => a,
            None => {
                debug!("eval recv error");
                let _ = stderr
                    .write(format!("exec: command failed\r\n").as_bytes())
                    .await;
                return ExecResponse::OrphanedImmediate(err::ERR_EINTR);
            }
        };

        let ctx = result.ctx;
        match result.status {
            EvalStatus::Executed { code, show_result } => {
                debug!("exec executed (code={})", code);
                if code != 0 && show_result {
                    let mut chars = String::new();
                    chars += err::exit_code_to_message(code);
                    chars += "\r\n";
                    let _ = stdout.write(chars.as_bytes()).await;
                }
                ExecResponse::Immediate(ctx, code)
            }
            EvalStatus::InternalError => {
                debug!("eval internal error");
                let _ = stderr.write("exec: internal error\r\n".as_bytes()).await;
                ExecResponse::Immediate(ctx, err::ERR_EINTR)
            }
            EvalStatus::MoreInput => {
                debug!("eval more input");
                let _ = stderr
                    .write("exec: incomplete command\r\n".as_bytes())
                    .await;
                ExecResponse::Immediate(ctx, err::ERR_EINVAL)
            }
            EvalStatus::Invalid => {
                debug!("eval invalid");
                let _ = stderr.write("exec: invalid command\r\n".as_bytes()).await;
                ExecResponse::Immediate(ctx, err::ERR_EINVAL)
            }
        }
    });
}

fn process_script(script: String, ctx: &EvalContext) -> String {
    // Currently the script engine requires a ";" on every line
    // TODO: make a more advanced script processor
    let script = script
        .replace("\r\n", "\n")
        .replace("\r", "\n")
        .replace("\n", ";\n")
        .replace(";;", ";")
        .replace(";\n;", ";")
        .to_string();

    // We also need to apply the environment variables
    let script = apply_env_vars(script, ctx);

    script
}

fn apply_env_vars(script: String, ctx: &EvalContext) -> String {
    // Replace all the environment variables with real values
    let mut script_parts = script.split("$").collect::<VecDeque<_>>();

    let mut ret = String::new();
    if let Some(first) = script_parts.pop_front() {
        ret.push_str(first);
    }

    for script_part in script_parts {
        // Find the end of the variable name
        // If it has a curly then just look for the next curcle
        let mut start = 0usize;
        let mut end1 = script_part.len();
        let mut end2 = script_part.len();
        if script_part.starts_with("{") {
            start = 1;
            let x: &[_] = &['}', '\n', '\r'];
            if let Some(e) = script_part.find(x) {
                end1 = e;
                end2 = e + 1;
            }
        } else {
            let x: &[_] = &[
                ' ', '\t', '.', ',', ';', ':', '\n', '\r', '\'', '"', '&', '*', '@', '!', '(', ')',
                '<', '>', '`', '/', '\\',
            ];
            if let Some(e) = script_part.find(x) {
                end1 = e;
                end2 = e;
            }
        }

        // Replace the variable with data
        if start < end1 {
            let name = &script_part[start..end1];
            if let Some(val) = ctx.env.get(name) {
                ret.push_str(val.as_str());
            }
        }

        // Add the remainder (if there is any)
        if end2 < script_part.len() {
            ret.push_str(&script_part[end2..]);
        }
    }
    ret
}
