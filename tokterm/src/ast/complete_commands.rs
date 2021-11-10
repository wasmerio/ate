use super::*;

#[derive(Debug, PartialEq)]
pub struct CompleteCommands<'a> {
    pub complete_commands: Vec<CompleteCommand<'a>>,
}

impl<'a> CompleteCommands<'a> {
    pub fn push(
        mut self: CompleteCommands<'a>,
        element: CompleteCommand<'a>,
    ) -> CompleteCommands<'a> {
        self.complete_commands.push(element);
        self
    }
}