#[macro_use]
extern crate lalrpop_util;

lalrpop_mod!(#[allow(clippy::all)] pub grammar);

mod common;
mod console;
mod eval;
mod glue;
mod cconst;
mod ast;
mod state;
mod environment;
mod builtins;
mod stdio;
mod stdout;
mod pool;
mod fd;
mod reactor;
mod poll;
mod err;
mod interval;
mod tty;
mod job;
mod wasi;
mod bin;
mod fs;

pub use glue::main;
pub use glue::start;