#[macro_use]
extern crate lalrpop_util;

lalrpop_mod!(#[allow(clippy::all)] pub grammar);

mod ast;
mod bin;
mod builtins;
mod cconst;
mod common;
mod console;
mod environment;
mod err;
mod eval;
mod fd;
mod fs;
mod glue;
mod interval;
mod job;
mod pipe;
mod poll;
mod pool;
mod reactor;
mod state;
mod stdio;
mod stdout;
mod tty;

pub use glue::main;
pub use glue::start;
