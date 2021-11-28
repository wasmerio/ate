#[macro_use]
extern crate lalrpop_util;

lalrpop_mod!(#[allow(clippy::all)] pub grammar);


mod console;
mod glue;
mod pool;
mod system;

use tokterm::reactor;
use tokterm::state;
use tokterm::stdio;
use tokterm::stdout;
use tokterm::tty;
use tokterm::interval;
use tokterm::job;
use tokterm::pipe;
use tokterm::poll;
use tokterm::environment;
use tokterm::err;
use tokterm::eval;
use tokterm::fd;
use tokterm::fs;
use tokterm::ast;
use tokterm::bin;
use tokterm::builtins;
use tokterm::bus;
use tokterm::cconst;
use tokterm::common;

pub use glue::main;
pub use glue::start;

pub(crate) use tokterm::wasmer;
pub(crate) use tokterm::wasmer_wasi;
pub(crate) use tokterm::wasmer_vfs;