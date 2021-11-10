mod cd;
mod export;
mod readonly;
mod about;
mod help;
mod unset;

use cd::*;
use export::*;
use readonly::*;
use about::*;
use help::*;
use unset::*;

use std::collections::HashMap;
use std::pin::Pin;
use std::future::Future;

use super::stdio::*;
use super::eval::EvalContext;

pub type Command = fn(&[String], &mut EvalContext, Stdio) -> Pin<Box<dyn Future<Output = i32>>>;

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
        b
    }

    fn insert(&mut self, key: &str, val: Command) {
        self.commands.insert(key.to_string(), val);
    }

    pub fn get(&self, key: &String) -> Option<&Command> {
        self.commands.get(key)
    }
}