mod about;
mod cd;
mod exec;
mod export;
mod help;
mod readonly;
mod unset;

use about::*;
use cd::*;
use exec::*;
use export::*;
use help::*;
use readonly::*;
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
        b.insert("exec", exec);
        b
    }

    fn insert(&mut self, key: &str, val: Command) {
        self.commands.insert(key.to_string(), val);
    }

    pub fn get(&self, key: &String) -> Option<&Command> {
        self.commands.get(key)
    }
}
