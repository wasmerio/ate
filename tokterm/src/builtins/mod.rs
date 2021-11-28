mod about;
mod cd;
mod export;
mod help;
mod pwd;
mod readonly;
mod reset;
mod source;
mod unset;

use about::*;
use cd::*;
use export::*;
use help::*;
use pwd::*;
use readonly::*;
use reset::*;
use source::*;
use unset::*;

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use super::eval::EvalContext;
use super::eval::ExecResponse;
use super::stdio::*;

pub type Command = fn(
    &[String],
    &mut EvalContext,
    Stdio,
) -> Pin<Box<dyn Future<Output = Result<ExecResponse, i32>>>>;

#[derive(Default)]
pub struct Builtins {
    commands: HashMap<String, Command>,
}

impl Builtins {
    pub fn new() -> Builtins {
        let mut b: Builtins = Default::default();
        b.insert("cd", cd);
        b.insert("export", export);
        b.insert("readonly", readonly);
        b.insert("unset", unset);
        b.insert("help", help);
        b.insert("about", about);
        b.insert("source", source);
        b.insert("pwd", pwd);
        b.insert("reset", reset);
        b
    }

    fn insert(&mut self, key: &str, val: Command) {
        self.commands.insert(key.to_string(), val);
    }

    pub fn get(&self, key: &String) -> Option<&Command> {
        self.commands.get(key)
    }
}
