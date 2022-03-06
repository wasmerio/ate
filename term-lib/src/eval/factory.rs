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
    pub abi: Arc<dyn ConsoleAbi>,
    pub env: Environment,
    pub job: Job,
    pub stdin: Fd,
    pub stdout: Fd,
    pub stderr: Fd,
    pub chroot: bool,
    pub working_dir: String,
    pub pre_open: Vec<String>,
    pub root: UnionFileSystem,
    pub compiler: Compiler,
}

impl SpawnContext {
    pub fn new(
        abi: Arc<dyn ConsoleAbi>,
        env: Environment,
        job: Job,
        stdin: Fd,
        stdout: Fd,
        stderr: Fd,
        chroot: bool,
        working_dir: String,
        pre_open: Vec<String>,
        root: UnionFileSystem,
        compiler: Compiler,
    ) -> SpawnContext {
        SpawnContext {
            abi,
            env,
            job,
            stdin,
            stdout,
            stderr,
            chroot,
            working_dir,
            pre_open,
            root,
            compiler,
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

    pub fn tty(&self) -> Tty {
        self.state.tty.clone()
    }

    pub fn stdout(&self) -> Stdout {
        self.state.stdout.clone()
    }

    pub fn stderr(&self) -> Fd {
        self.state.stderr.clone()
    }

    pub fn log(&self) -> Fd {
        self.state.log.clone()
    }

    pub fn stdio(&self, stdin: Fd) -> Stdio {
        Stdio {
            stdin,
            stdout: self.stdout().fd.clone(),
            stderr: self.stderr(),
            log: self.log(),
            tty: self.tty(),
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
            abi: ctx.abi.clone(),
            env: ctx.env,
            bins: self.state.bins.clone(),
            reactor: self.state.reactor.clone(),
            chroot: ctx.chroot,
            working_dir: ctx.working_dir,
            last_return: 0u32,
            pre_open: ctx.pre_open,
            stdio,
            root: ctx.root,
            exec_factory: self.clone(),
            job: ctx.job,
            compiler: ctx.compiler,
        };

        ctx
    }

    pub fn eval(&self, cmd: String, ctx: SpawnContext) -> mpsc::Receiver<EvalResult> {
        crate::eval::eval(cmd, self.create_context(ctx))
    }
}
