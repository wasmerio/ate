#![allow(dead_code)]
#![allow(unused)]

pub(crate) mod andor_list;
pub(crate) mod complete_command;
pub(crate) mod eval_arg;
pub(crate) mod exec;
pub(crate) mod exec_pipeline;
pub(crate) mod factory;
pub(crate) mod load_bin;
pub(crate) mod process;
pub(crate) mod runtime;
pub(crate) mod bus_feeder;
pub(crate) mod bus_listener;
pub(crate) mod bus_handle;

pub use andor_list::*;
pub use complete_command::*;
use derivative::Derivative;
pub use eval_arg::*;
pub use exec::*;
pub use exec_pipeline::*;
pub use factory::*;
pub use load_bin::*;
pub use process::*;
pub use runtime::*;
pub use bus_feeder::*;
pub use bus_listener::*;
pub use bus_handle::*;

use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::watch;
use tokio::sync::RwLock;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

#[cfg(feature = "wasmer-compiler")]
use {crate::wasmer::Universal, crate::wasmer_compiler::CompilerConfig};
use crate::wasmer::{Store};
#[cfg(feature = "wasmer-compiler-cranelift")]
use crate::wasmer_compiler_cranelift::Cranelift;
#[cfg(feature = "wasmer-compiler-llvm")]
use crate::wasmer_compiler_llvm::LLVM;
#[cfg(feature = "wasmer-compiler-singlepass")]
use crate::wasmer_compiler_singlepass::Singlepass;

use crate::api::*;
use crate::ast;
use crate::environment::Environment;

use super::ast::*;
use super::bin_factory::*;
use super::builtins::*;
use super::common::*;
use super::err;
use super::fd::*;
use super::fs::*;
use super::grammar;
use super::job::*;
use super::reactor::*;
use super::state::*;
use super::stdio::*;
use super::bus::LaunchEnvironment;
use super::grammar::ast::Redirect;

#[derive(Debug)]
pub enum EvalStatus {
    Executed { code: u32, show_result: bool },
    MoreInput,
    Invalid,
    InternalError,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct EvalResult {
    pub ctx: EvalContext,
    #[derivative(Debug = "ignore")]
    pub status: EvalStatus,
}

impl EvalResult {
    pub fn new(ctx: EvalContext, status: EvalStatus) -> EvalResult {
        EvalResult { ctx, status }
    }

    pub fn raw(self) -> u32 {
        match &self.status {
            EvalStatus::Executed { code, .. } => *code,
            _ => 1,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Compiler {
    #[cfg(feature = "singlepass")]
    Singlepass,
    #[cfg(feature = "llvm")]
    LLVM,
    #[cfg(feature = "cranelift")]
    Cranelift,
    #[cfg(feature = "js")]
    Browser
}

impl Default
for Compiler
{
    #[cfg(feature = "llvm")]
    fn default() -> Self {
        Self::LLVM
    }

    #[cfg(all(not(feature = "llvm"), feature = "cranelift"))]
    fn default() -> Self {
        Self::Cranelift
    }

    #[cfg(all(not(feature = "llvm"), not(feature = "cranelift"), feature = "singlepass"))]
    fn default() -> Self {
        Self::Singlepass
    }

    #[cfg(all(not(feature = "llvm"), not(feature = "cranelift"), not(feature = "singlepass"), feature = "js"))]
    fn default() -> Self {
        Self::Browser
    }
}

impl Compiler
{
    #[cfg(feature = "wasmer-compiler")]
    pub fn new_store(&self) -> Store
    {
        // Build the features list
        let mut features = wasmer_compiler::Features::new();
        features.threads(true);
        features.memory64(true);
        #[cfg(feature = "singlepass")]
        if let Compiler::Singlepass = self {
            features.multi_value(false);
        }

        // Choose the right compiler
        let store = match self {
            #[cfg(feature = "cranelift")]
            Compiler::Cranelift => {
                let compiler = Cranelift::default();
                Store::new(&Universal::new(compiler)
                    .features(features)
                    .engine())
            }
            #[cfg(feature = "llvm")]
            Compiler::LLVM => {
                let compiler = LLVM::default();
                Store::new(&Universal::new(compiler)
                    .features(features)
                    .engine())
            }
            #[cfg(feature = "singlepass")]
            Compiler::Singlepass => {
                let compiler = Singlepass::default();
                Store::new(&Universal::new(compiler)
                    .features(features)
                    .engine())
            }
            #[cfg(feature = "js")]
            Compiler::Browser => {
                Store::default()
            }
        };
        store
    }

    #[cfg(not(feature = "wasmer-compiler"))]
    pub fn new_store(&self) -> Store
    {
        Store::default()
    }
}

impl std::str::FromStr for Compiler {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "default" => Ok(Default::default()),
            #[cfg(feature = "singlepass")]
            "singlepass" => Ok(Compiler::Singlepass),
            #[cfg(feature = "cranelift")]
            "cranelift" => Ok(Compiler::Cranelift),
            #[cfg(feature = "llvm")]
            "llvm" => Ok(Compiler::LLVM),
            _ => {
                let mut msg = "valid values are 'default'".to_string();
                #[cfg(feature = "singlepass")]
                msg.push_str(", 'singlepass'");
                #[cfg(feature = "llvm")]
                msg.push_str(", 'llvm'");
                #[cfg(feature = "cranelift")]
                msg.push_str(", 'cranelift'");
                Err(msg)
            }
        }
    }
}

impl std::fmt::Display
for Compiler
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "singlepass")]
            Compiler::Singlepass => write!(f, "singlepass"),
            #[cfg(feature = "cranelift")]
            Compiler::Cranelift => write!(f, "cranelift"),
            #[cfg(feature = "llvm")]
            Compiler::LLVM => write!(f, "llvm"),
            #[cfg(feature = "js")]
            Compiler::Browser => write!(f, "browser"),
        }
    }
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct EvalContext {
    #[derivative(Debug = "ignore")]
    pub system: System,
    #[derivative(Debug = "ignore")]
    pub abi: Arc<dyn ConsoleAbi>,
    #[derivative(Debug = "ignore")]
    pub env: Environment,
    #[derivative(Debug = "ignore")]
    pub bins: BinFactory,
    pub last_return: u32,
    #[derivative(Debug = "ignore")]
    pub reactor: Arc<RwLock<Reactor>>,
    pub chroot: bool,
    pub working_dir: String,
    pub pre_open: Vec<String>,
    #[derivative(Debug = "ignore")]
    pub stdio: Stdio,
    #[derivative(Debug = "ignore")]
    pub root: UnionFileSystem,
    #[derivative(Debug = "ignore")]
    pub exec_factory: EvalFactory,
    #[derivative(Debug = "ignore")]
    pub job: Job,
    pub compiler: Compiler,
    pub extra_args: Vec<String>,
    pub extra_redirects: Vec<Redirect>,
}

impl EvalContext {
    pub fn launch_env(&self) -> LaunchEnvironment {
        LaunchEnvironment {
            abi: self.abi.clone(),
            inherit_stderr: self.stdio.stderr.downgrade(),
            inherit_stdout: self.stdio.stdout.downgrade(),
            inherit_stdin: self.stdio.stdin.downgrade(),
            inherit_log: self.stdio.log.downgrade(),
        }
    }
}

pub(crate) fn eval(cmd: String, mut ctx: EvalContext) -> mpsc::Receiver<EvalResult> {
    let system = ctx.system;
    let builtins = Builtins::new();
    let parser = grammar::programParser::new();

    let (tx, rx) = mpsc::channel(1);

    let work = {
        async move {
            match parser.parse(cmd.as_str()) {
                Ok(program) => {
                    let mut show_result = false;
                    let mut ret = 0;
                    for cc in program.commands.complete_commands {
                        let (c, r) = complete_command(ctx, &builtins, &cc, &mut show_result).await;
                        ctx = c;
                        ret = r;
                    }
                    tx.send(EvalResult::new(
                        ctx,
                        EvalStatus::Executed {
                            code: ret,
                            show_result,
                        },
                    ))
                    .await;
                }
                Err(e) => match e {
                    grammar::ParseError::UnrecognizedToken {
                        token: _,
                        expected: _,
                    } => {
                        tx.send(EvalResult::new(ctx, EvalStatus::MoreInput)).await;
                    }
                    grammar::ParseError::UnrecognizedEOF {
                        location: _,
                        expected: _,
                    } => {
                        tx.send(EvalResult::new(ctx, EvalStatus::MoreInput)).await;
                    }
                    _ => {
                        tx.send(EvalResult::new(ctx, EvalStatus::Invalid)).await;
                    }
                },
            }
        }
    };

    #[cfg(target_family = "wasm")]
    system.fork_local(work);
    #[cfg(not(target_family = "wasm"))]
    system.fork_shared(move || work);
    rx
}
