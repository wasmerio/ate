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
use tokio::sync::RwLock;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

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

pub enum EvalPlan {
    Executed {
        code: u32,
        ctx: EvalContext,
        show_result: bool,
    },
    MoreInput,
    Invalid,
    InternalError,
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
    pub new_pwd: Option<String>,
    pub pre_open: Vec<String>,
    pub input: String,
    pub stdio: Stdio,
    pub root: UnionFileSystem,
    pub new_mounts: Vec<MountPoint>,
    pub exec_factory: EvalFactory,
    pub job: Job,
    pub compiler: Compiler,
}

pub(crate) fn eval(mut ctx: EvalContext) -> mpsc::Receiver<EvalPlan> {
    let system = ctx.system;
    let builtins = Builtins::new();
    let parser = grammar::programParser::new();

    let (tx, rx) = mpsc::channel(1);
    system.fork_local(async move {
        let input = ctx.input.clone();
        match parser.parse(input.as_str()) {
            Ok(program) => {
                let mut show_result = false;
                let mut ret = 0;
                for cc in program.commands.complete_commands {
                    ret = complete_command(&mut ctx, &builtins, &cc, &mut show_result).await;
                    ctx.last_return = ret;
                }
                tx.send(EvalPlan::Executed {
                    code: ret,
                    ctx,
                    show_result,
                })
                .await;
            }
            Err(e) => match e {
                grammar::ParseError::UnrecognizedToken {
                    token: _,
                    expected: _,
                } => {
                    tx.send(EvalPlan::MoreInput).await;
                }
                grammar::ParseError::UnrecognizedEOF {
                    location: _,
                    expected: _,
                } => {
                    tx.send(EvalPlan::MoreInput).await;
                }
                _ => {
                    tx.send(EvalPlan::Invalid).await;
                }
            },
        }
    });
    rx
}
