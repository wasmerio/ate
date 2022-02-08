mod about;
mod cd;
mod exit;
mod export;
mod help;
mod mount;
mod pwd;
mod readonly;
mod reset;
mod source;
mod umount;
mod unset;
mod wax;

use about::*;
use cd::*;
use exit::*;
use export::*;
use help::*;
use mount::*;
use pwd::*;
use readonly::*;
use reset::*;
use source::*;
use umount::*;
use unset::*;
use wax::*;

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use super::eval::EvalContext;
use super::eval::ExecResponse;
use super::stdio::*;

pub type Command = fn(&[String], EvalContext, Stdio) -> Pin<Box<dyn Future<Output = ExecResponse> + Send>>;

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
        b.insert("mount", mount);
        b.insert("umount", umount);
        b.insert("unmount", umount);
        b.insert("wax", wax);
        b.insert("exit", exit);
        b.insert("quit", exit);
        b
    }

    fn insert(&mut self, key: &str, val: Command) {
        self.commands.insert(key.to_string(), val);
    }

    pub fn get(&self, key: &String) -> Option<&Command> {
        self.commands.get(key)
    }
}
