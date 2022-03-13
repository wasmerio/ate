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

pub use andor_list::*;
pub use complete_command::*;
pub use eval_arg::*;
pub use exec::*;
pub use exec_pipeline::*;
pub use factory::*;
pub use load_bin::*;
pub use process::*;

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

pub enum EvalStatus {
    Executed { code: u32, show_result: bool },
    MoreInput,
    Invalid,
    InternalError,
}

pub struct EvalResult {
    pub ctx: EvalContext,
    pub status: EvalStatus,
}

impl EvalResult {
    pub fn new(ctx: EvalContext, status: EvalStatus) -> EvalResult {
        EvalResult { ctx, status }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Compiler {
    Default,
    #[cfg(feature = "singlepass")]
    Singlepass,
    #[cfg(feature = "llvm")]
    LLVM,
    #[cfg(feature = "cranelift")]
    Cranelift,
}

impl Compiler
{
    #[cfg(feature = "wasmer-compiler")]
    pub fn new_store(&self) -> Store
    {
        // Choose the right compiler
        let store = match self {
            #[cfg(feature = "cranelift")]
            Compiler::Cranelift => {
                let compiler = Cranelift::default();
                Store::new(&Universal::new(compiler).engine())
            }
            #[cfg(feature = "llvm")]
            Compiler::LLVM => {
                let compiler = LLVM::default();
                Store::new(&Universal::new(compiler).engine())
            }
            #[cfg(feature = "singlepass")]
            Compiler::Singlepass => {
                let compiler = Singlepass::default();
                Store::new(&Universal::new(compiler).engine())
            }
            _ => Store::default(),
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
            "default" => Ok(Compiler::Default),
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
            Compiler::Default => write!(f, "default")
        }
    }
}

#[derive(Clone)]
pub struct EvalContext {
    pub system: System,
    pub abi: Arc<dyn ConsoleAbi>,
    pub env: Environment,
    pub bins: BinFactory,
    pub last_return: u32,
    pub reactor: Arc<RwLock<Reactor>>,
    pub chroot: bool,
    pub working_dir: String,
    pub pre_open: Vec<String>,
    pub stdio: Stdio,
    pub root: UnionFileSystem,
    pub exec_factory: EvalFactory,
    pub job: Job,
    pub compiler: Compiler,
    pub extra_args: Vec<String>,
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

    #[cfg(target_arch = "wasm32")]
    system.fork_local(work);
    #[cfg(not(target_arch = "wasm32"))]
    system.fork_shared(move || work);
    rx
}
