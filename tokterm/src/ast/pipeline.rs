use super::*;

#[derive(Debug, PartialEq)]
pub struct Pipeline<'a> {
    pub commands: Vec<Command<'a>>,
    pub negated: bool,
}

impl<'a> Pipeline<'a> {
    pub fn new(cmd: Command<'a>) -> Pipeline<'a> {
        Pipeline {
            commands: vec![cmd],
            negated: false,
        }
    }

    pub fn negate(mut self) -> Pipeline<'a> {
        self.negated = !self.negated;
        self
    }

    pub fn push(mut self, cmd: Command<'a>) -> Pipeline<'a> {
        self.commands.push(cmd);
        self
    }
}
