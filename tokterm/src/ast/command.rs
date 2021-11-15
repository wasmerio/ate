use super::*;

#[derive(Debug, PartialEq)]
pub enum Command<'a> {
    Simple {
        assign: Vec<&'a str>,
        cmd: Arg<'a>,
        args: Vec<Arg<'a>>,
        //redirect: Vec<Redirect<'a>>,
    },
}
