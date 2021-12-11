use std::ops::Deref;

use super::*;
use crate::ast;
use crate::pipe::*;
use crate::wasmer_vfs::FileSystem;
use crate::wasmer_vfs::FsError;
use tokio::select;

pub(super) async fn exec_pipeline<'a>(
    ctx: &mut EvalContext,
    builtins: &Builtins,
    exec_sync: bool,
    show_result: &mut bool,
    pipeline: &'a ast::Pipeline<'a>,
) -> i32 {
    let mut child_list = Vec::new();
    let mut final_return: Option<i32> = None;
    {
        let mut next_stdin = ctx.stdio.stdin.clone();
        let mut cur_stdin = ctx.stdio.stdin.clone();
        let mut cur_stdout = ctx.stdio.stdout.clone();
        let mut cur_stderr = ctx.stdio.stderr.clone();
        let end_stdout = ctx.stdio.stdout.clone();

        for i in 0..pipeline.commands.len() {
            let command = &pipeline.commands[i];
            match command {
                ast::Command::Simple {
                    assign,
                    cmd,
                    args,
                    redirect,
                } => {
                    let parsed_cmd = match cmd {
                        ast::Arg::Arg(s) => eval_arg(&ctx.env, ctx.last_return, *s),
                        ast::Arg::Backquote(_quoted_args) => String::new(),
                    };
                    let mut parsed_args: Vec<String> = vec![parsed_cmd.clone()];
                    parsed_args.extend(args.iter().map(|a| match a {
                        ast::Arg::Arg(s) => eval_arg(&ctx.env, ctx.last_return, *s),
                        ast::Arg::Backquote(_quoted_args) => String::new(),
                    }));
                    let parsed_env: Vec<String> = assign.iter().map(|a| a.to_string()).collect();

                    cur_stdin = next_stdin.clone();
                    if i + 1 < pipeline.commands.len() {
                        let (w, r) = pipe(ReceiverMode::Stream, end_stdout.is_tty());
                        next_stdin = r;
                        cur_stdout = w;
                    } else {
                        cur_stdout = end_stdout.clone();
                    }

                    let mut stdio = Stdio {
                        stdin: cur_stdin.clone(),
                        stdout: cur_stdout.clone(),
                        stderr: cur_stderr.clone(),
                        log: ctx.stdio.log.clone(),
                        tty: ctx.stdio.tty.clone(),
                    };

                    debug!("exec {}", parsed_cmd);
                    match exec::exec(
                        ctx,
                        builtins,
                        &parsed_cmd,
                        &parsed_args,
                        &parsed_env,
                        show_result,
                        stdio,
                        &redirect,
                    )
                    .await
                    {
                        Ok(ExecResponse::Immediate(ret)) => final_return = Some(ret),
                        Ok(ExecResponse::Process(process, process_result)) => {
                            child_list.push((process, process_result));
                        }
                        Err(err) => {
                            *show_result = true;
                            final_return = Some(err);
                        }
                    }
                }
            }
        }
    }

    for (child, child_result) in child_list.iter() {
        debug!(
            "process (pid={}) added to job (id={})",
            child.pid, ctx.job.id
        );
        ctx.job.job_list_tx.send(child.pid).await;
    }

    if exec_sync {
        for (child, child_result) in child_list.into_iter().rev() {
            let result = child_result
                .join()
                .await
                .unwrap_or_else(|| err::ERR_ECONNABORTED);
            debug!(
                "process (pid={}) finished (exit_code={})",
                child.pid, result
            );
            final_return.get_or_insert(result);
        }
    }

    final_return.map_or_else(|| 0, |a| a)
}
