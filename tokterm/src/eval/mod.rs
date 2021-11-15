#![allow(dead_code)]
#![allow(unused)]

pub(crate) mod andor_list;
pub(crate) mod complete_command;
pub(crate) mod eval_arg;
pub(crate) mod exec;
pub(crate) mod exec_pipeline;
pub(crate) mod load_bin;
pub(crate) mod process;

pub use andor_list::*;
pub use complete_command::*;
pub use eval_arg::*;
pub use exec::*;
pub use exec_pipeline::*;
pub use load_bin::*;
pub use process::*;

use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::RwLock;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::ast;
use crate::environment::Environment;

use super::ast::*;
use super::bin::*;
use super::builtins::*;
use super::common::*;
use super::err;
use super::fd::*;
use super::grammar;
use super::job::*;
use super::pool::*;
use super::reactor::*;
use super::state::*;
use super::stdio::*;

pub enum EvalPlan {
    Executed {
        code: i32,
        ctx: EvalContext,
        show_result: bool,
    },
    MoreInput,
    Invalid,
    InternalError,
}

#[derive(Clone)]
pub struct EvalContext {
    pub env: Environment,
    pub bins: BinFactory,
    pub job_list: mpsc::Sender<Pid>,
    pub last_return: i32,
    pub reactor: Arc<RwLock<Reactor>>,
    pub pool: ThreadPool,
    pub path: String,
    pub input: String,
    pub console: Arc<Mutex<ConsoleState>>,
    pub stdio: Stdio,
}

pub(crate) async fn eval(mut ctx: EvalContext) -> oneshot::Receiver<EvalPlan> {
    let builtins = Builtins::new();
    let parser = grammar::programParser::new();

    let (tx, rx) = oneshot::channel();
    wasm_bindgen_futures::spawn_local(async move {
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
                });
            }
            Err(e) => match e {
                lalrpop_util::ParseError::UnrecognizedToken {
                    token: _,
                    expected: _,
                } => {
                    tx.send(EvalPlan::MoreInput);
                }
                lalrpop_util::ParseError::UnrecognizedEOF {
                    location: _,
                    expected: _,
                } => {
                    tx.send(EvalPlan::MoreInput);
                }
                _ => {
                    tx.send(EvalPlan::Invalid);
                }
            },
        }
    });
    rx
}
