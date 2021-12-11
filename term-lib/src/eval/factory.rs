use tokio::sync::oneshot;

use crate::api::*;
use crate::bin_factory::*;
use crate::eval::*;
use crate::fd::*;
use crate::pipe::*;
use crate::state::*;
use crate::stdout::*;
use crate::tty::*;

pub struct SpawnContext {
    pub cmd: String,
    pub env: Environment,
    pub job: Job,
    pub stdin: Fd,
    pub stdout: Fd,
    pub stderr: Fd,
    pub working_dir: String,
    pub pre_open: Vec<String>,
    pub root: UnionFileSystem,
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

pub struct EvalFactoryState {
    pub bins: BinFactory,
    pub tty: Tty,
    pub reactor: Arc<RwLock<Reactor>>,
    pub stdout: Stdout,
    pub stderr: Fd,
    pub log: Fd,
}

#[derive(Clone)]
pub struct EvalFactory {
    pub(crate) state: Arc<EvalFactoryState>,
}

impl EvalFactory {
    pub fn new(
        bins: BinFactory,
        tty: Tty,
        reactor: Arc<RwLock<Reactor>>,
        stdout: Stdout,
        stderr: Fd,
        log: Fd,
    ) -> EvalFactory {
        EvalFactory {
            state: Arc::new(EvalFactoryState {
                bins,
                tty,
                reactor,
                stdout,
                stderr,
                log,
            }),
        }
    }

    pub fn create_context(&self, ctx: SpawnContext) -> crate::eval::EvalContext {
        // Build the standard IO
        let stdio = Stdio {
            stdin: ctx.stdin,
            stdout: ctx.stdout,
            stderr: ctx.stderr,
            log: self.state.log.clone(),
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
            pre_open: ctx.pre_open,
            input: ctx.cmd,
            stdio,
            root: ctx.root,
            new_mounts: Vec::new(),
            exec_factory: self.clone(),
            job: ctx.job,
        };

        ctx
    }

    pub fn eval(&self, ctx: SpawnContext) -> mpsc::Receiver<EvalPlan> {
        crate::eval::eval(self.create_context(ctx))
    }
}
