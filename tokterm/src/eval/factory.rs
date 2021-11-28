use tokio::sync::oneshot;

use crate::api::*;
use crate::bin::*;
use crate::eval::*;
use crate::fd::*;
use crate::pipe::*;
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
    pre_open: Vec<String>,
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
        pre_open: Vec<String>,
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
            pre_open,
            root,
        }
    }
}

pub struct ExecFactoryState {
    pub bins: BinFactory,
    pub tty: Tty,
    pub reactor: Arc<RwLock<Reactor>>,
    pub stdout: Stdout,
    pub stderr: Fd,
}

#[derive(Clone)]
pub struct ExecFactory {
    pub(crate) state: Arc<ExecFactoryState>,
}

impl ExecFactory {
    pub fn new(
        bins: BinFactory,
        tty: Tty,
        reactor: Arc<RwLock<Reactor>>,
        stdout: Stdout,
        stderr: Fd,
    ) -> ExecFactory {
        ExecFactory {
            state: Arc::new(ExecFactoryState {
                bins,
                tty,
                reactor,
                stdout,
                stderr,
            }),
        }
    }

    pub async fn spawn(&self, ctx: SpawnContext) -> mpsc::Receiver<EvalPlan> {
        // Build the standard IO
        let stdio = Stdio {
            stdin: ctx.stdin,
            stdout: ctx.stdout,
            stderr: ctx.stderr,
            tty: self.state.tty.clone(),
        };

        // Create the evaluation context
        let ctx = crate::eval::EvalContext {
            system: System::default(),
            env: ctx.env,
            bins: self.state.bins.clone(),
            last_return: 0i32,
            reactor: self.state.reactor.clone(),
            path: ctx.working_dir,
            pre_open: ctx.pre_open.clone(),
            input: ctx.cmd,
            stdio,
            root: ctx.root.clone(),
            exec_factory: self.clone(),
            job: ctx.job.clone(),
        };

        eval(ctx).await
    }
}
