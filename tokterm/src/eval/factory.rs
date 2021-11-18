use tokio::sync::oneshot;

use crate::bin::*;
use crate::eval::*;
use crate::fd::*;
use crate::pipe::*;
use crate::pool::*;
use crate::state::*;
use crate::stdout::*;
use crate::tty::*;

pub struct SpawnContext {
    cmd: String,
    env: Environment,
    job: Job,
    stdin: Fd,
    stdout: Fd,
    stderr: Fd,
    working_dir: String,
    root: UnionFileSystem,
}

impl SpawnContext {
    pub fn new(
        cmd: String,
        env: Environment,
        job: Job,
        stdin: Fd,
        stdout: Fd,
        stderr: Fd,
        working_dir: String,
        root: UnionFileSystem,
    ) -> SpawnContext {
        SpawnContext {
            cmd,
            env,
            job,
            stdin,
            stdout,
            stderr,
            working_dir,
            root,
        }
    }
}

#[derive(Clone)]
pub struct ExecFactory {
    pub(crate) bins: BinFactory,
    pub(crate) tty: Tty,
    pub(crate) pool: ThreadPool,
    pub(crate) reactor: Arc<RwLock<Reactor>>,
    pub(crate) stdout: Stdout,
    pub(crate) stderr: Fd,
}

impl ExecFactory {
    pub fn new(
        bins: BinFactory,
        tty: Tty,
        pool: ThreadPool,
        reactor: Arc<RwLock<Reactor>>,
        stdout: Stdout,
        stderr: Fd,
    ) -> ExecFactory {
        ExecFactory {
            bins,
            tty,
            pool,
            reactor,
            stdout,
            stderr,
        }
    }

    pub async fn spawn(&self, ctx: SpawnContext) -> oneshot::Receiver<EvalPlan> {
        // Build the standard IO
        let stdio = Stdio {
            stdin: ctx.stdin,
            stdout: ctx.stdout,
            stderr: ctx.stderr,
            tty: self.tty.clone(),
        };

        // Create the evaluation context
        let ctx = crate::eval::EvalContext {
            env: ctx.env,
            bins: self.bins.clone(),
            last_return: 0i32,
            reactor: self.reactor.clone(),
            pool: self.pool.clone(),
            path: ctx.working_dir,
            input: ctx.cmd,
            stdio,
            root: ctx.root.clone(),
            exec_factory: self.clone(),
            job: ctx.job.clone(),
        };

        eval(ctx).await
    }
}
