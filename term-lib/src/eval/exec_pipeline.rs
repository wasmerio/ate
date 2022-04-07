use std::ops::Deref;

use super::*;
use crate::ast;
use crate::pipe::*;
use crate::wasmer_vfs::FileSystem;
use crate::wasmer_vfs::FsError;
use tokio::select;

pub(super) async fn exec_pipeline<'a>(
    mut ctx: EvalContext,
    builtins: &Builtins,
    exec_sync: bool,
    show_result: &mut bool,
    pipeline: &'a ast::Pipeline<'a>,
) -> (EvalContext, u32) {
    let mut child_list = Vec::new();
    let mut final_return: Option<u32> = None;
    {
        let mut next_stdin = ctx.stdio.stdin.clone();
        let mut cur_stdin = ctx.stdio.stdin.clone();
        let mut cur_stdout = ctx.stdio.stdout.clone();
        let mut cur_stderr = ctx.stdio.stderr.clone();
        let end_stdout = ctx.stdio.stdout.clone();

        for i in 0..pipeline.commands.len() {
            let is_last = i == pipeline.commands.len() - 1;
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
                    parsed_args.extend(ctx.extra_args.clone().into_iter());
                    let parsed_env: Vec<String> = assign.iter().map(|a| a.to_string()).collect();

                    let mut parsed_redirects = redirect.clone().into_iter().collect::<Vec<_>>();
                    parsed_redirects.extend(ctx.extra_redirects.clone().into_iter());

                    cur_stdin = next_stdin.clone();
                    if i + 1 < pipeline.commands.len() {
                        let (mut w, mut r) = pipe(ReceiverMode::Stream, end_stdout.flag());
                        r.set_flag(FdFlag::Stdin(false));
                        w.set_flag(FdFlag::Stdout(false));
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
                        ctx.clone(),
                        builtins,
                        &parsed_cmd,
                        &parsed_args,
                        &parsed_env,
                        show_result,
                        stdio,
                        &parsed_redirects,
                    )
                    .await
                    {
                        Ok(ExecResponse::Immediate(c, ret)) => {
                            ctx = c;
                            final_return = Some(ret);
                        }
                        Ok(ExecResponse::OrphanedImmediate(ret)) => {
                            final_return = Some(ret);
                        }
                        Ok(ExecResponse::Process(process, process_result)) => {
                            child_list.push((process, process_result, is_last));
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

    for (child, child_result, _) in child_list.iter() {
        debug!(
            "process (pid={}) added to job (id={})",
            child.pid, ctx.job.id
        );
        ctx.job.job_list_tx.send(child.pid).await;
    }

    if exec_sync {
        for (child, child_result, is_last) in child_list.into_iter().rev() {
            let (c, result) = child_result
                .await
                .map(|(c, r)| (Some(c), r))
                .unwrap_or_else(|| (None, err::ERR_ECONNABORTED));
            debug!(
                "process (pid={}) finished (exit_code={})",
                child.pid, result
            );
            final_return.get_or_insert(result);
            if let Some(c) = c {
                if is_last {
                    ctx = c;
                }
            }
        }
    }

    (ctx, final_return.map_or_else(|| 0, |a| a))
}
