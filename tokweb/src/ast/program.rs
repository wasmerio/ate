use super::*;

#[derive(Debug, PartialEq)]
pub struct Program<'a> {
    pub commands: CompleteCommands<'a>,
}
