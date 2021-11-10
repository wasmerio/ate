use crate::ast;
use super::*;

pub(super) async fn exec_pipeline<'a>(
    ctx: &mut EvalContext,
    builtins: &Builtins,
    exec_sync: bool,
    show_result: &mut bool,
    pipeline: &'a ast::Pipeline<'a>
) -> i32
{
    debug!("eval (stdin={}, stdout={}, stderr={})", ctx.stdio.stdin.raw.id, ctx.stdio.stdout.raw.id, ctx.stdio.stderr.raw.id);

    let mut child_list: Vec<Process> = Vec::new();
    let mut final_return: Option<i32> = None;
    {
        let (mut next_stdin, mut cur_stdin, mut cur_stdout, mut end_stdout, mut cur_stderr) = {
            let reactor = ctx.reactor.read().await;
            let next_stdin: Fd = reactor.fd(ctx.stdio.stdin.raw);
            let cur_stdin: Fd = reactor.fd(ctx.stdio.stdin.raw);
            let cur_stdout: Fd = reactor.fd(ctx.stdio.stdout.raw);
            let end_stdout: Fd = reactor.fd(ctx.stdio.stdout.raw);
            let cur_stderr: Fd = reactor.fd(ctx.stdio.stderr.raw);
            (next_stdin, cur_stdin, cur_stdout, end_stdout, cur_stderr)
        };

        for i in 0..pipeline.commands.len() {
            let command = &pipeline.commands[i];
            match command {
                ast::Command::Simple { assign, cmd, args } => {
                    let parsed_cmd = match cmd {
                        ast::Arg::Arg(s) => eval_arg(&ctx.env, ctx.last_return, *s),
                        ast::Arg::Backquote(_quoted_args) => String::new(),
                    };
                    let mut parsed_args: Vec<String> = vec![parsed_cmd.clone()];
                    parsed_args.extend(args.iter().map(|a| match a {
                        ast::Arg::Arg(s) => eval_arg(&ctx.env, ctx.last_return, *s),
                        ast::Arg::Backquote(_quoted_args) => String::new(),
                    }));
                    let parsed_env: Vec<String> =
                        assign.iter().map(|a| a.to_string()).collect();

                    cur_stdin = next_stdin.clone();
                    if i + 1 < pipeline.commands.len() {
                        let mut reactor = ctx.reactor.write().await;
                        let (w, r) = match reactor.pipe(ReceiverMode::Stream) {
                            Ok(a) => a,
                            Err(err) => {
                                return err::ERR_EMFILE;
                            }
                        };
                        debug!("pipe created {} -> {}", w.id, r.id);
                        next_stdin = reactor.fd(r);
                        cur_stdout = reactor.fd(w);
                        reactor.remove_pipe(w);
                        reactor.remove_pipe(r);
                    } else {
                        cur_stdout = end_stdout.clone();
                    }

                    let stdio = Stdio {
                        stdin: cur_stdin.clone(),
                        stdout: cur_stdout.clone(),
                        stderr: cur_stderr.clone(),
                        tty: ctx.stdio.tty.clone(),
                        tok: ctx.stdio.tok.clone(),
                        root: ctx.stdio.root.clone(),
                    };

                    debug!("exec {} (stdin={}, stdout={}, stderr={})", parsed_cmd, stdio.stdin.raw.id, stdio.stdout.raw.id, stdio.stderr.raw.id);

                    match exec::exec (
                        ctx,
                        builtins,
                        &parsed_cmd,
                        &parsed_args,
                        &parsed_env,
                        show_result,
                        stdio
                    ).await
                    {
                        Ok(ExecResponse::Immediate(ret)) => {
                            final_return = Some(ret)
                        }
                        Ok(ExecResponse::Process(process)) => {
                            child_list.push(process);
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

    for child in child_list.iter() {
        debug!("process (pid={}) added to job", child.pid);
        ctx.job_list.send(child.pid).await;
    }

    if exec_sync {
        for child in child_list.iter_mut().rev() {
            let result = child.wait_for_exit().await;
            debug!("process (pid={}) finished (exit_code={})", child.pid, result);
            final_return.get_or_insert(result);
        }
    }

    final_return.map_or_else(|| 0, |a| a)
}