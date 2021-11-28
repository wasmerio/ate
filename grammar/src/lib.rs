#[allow(unused_imports)]
#[macro_use]
extern crate lalrpop_util;

lalrpop_mod!(#[allow(clippy::all)] grammar);

pub mod ast;

pub use grammar::*;
pub use lalrpop_util::*;