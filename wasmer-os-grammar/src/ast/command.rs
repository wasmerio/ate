use super::*;

#[derive(Debug, PartialEq)]
pub enum Command<'a> {
    Simple {
        assign: Vec<&'a str>,
        cmd: Arg<'a>,
        args: Vec<Arg<'a>>,
        redirect: Vec<Redirect>,
    },
}

impl<'a> Command<'a> {
    pub fn redirect(&mut self) -> &mut Vec<Redirect> {
        match self {
            Command::Simple { redirect, .. } => redirect,
        }
    }
}
