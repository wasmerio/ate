use super::*;

#[derive(Debug, PartialEq)]
pub struct CompleteCommand<'a> {
    pub and_ors: Vec<(TermOp, AndOr<'a>)>,
}

impl<'a> CompleteCommand<'a> {
    pub fn push(mut self, op: TermOp, element: AndOr<'a>) -> CompleteCommand<'a> {
        // update the TermOp of the previous list entry
        self.update_last(op);
        // add the new entry and assume it ends with a semicolon
        self.and_ors.push((TermOp::Semi, element));
        self
    }

    pub fn update_last(&mut self, op: TermOp) {
        if let Some((_, e)) = self.and_ors.pop() {
            self.and_ors.push((op, e));
        }
    }
}
