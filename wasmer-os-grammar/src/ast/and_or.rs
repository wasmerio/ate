use super::*;

#[derive(Debug, PartialEq)]
pub struct AndOr<'a> {
    pub pipelines: Vec<(AndOrOp, Pipeline<'a>)>,
}

impl<'a> AndOr<'a> {
    pub fn push(mut self, op: AndOrOp, element: Pipeline<'a>) -> AndOr<'a> {
        if let Some((_, e)) = self.pipelines.pop() {
            self.pipelines.push((op, e));
        }
        self.pipelines.push((AndOrOp::And, element));
        self
    }
}

#[derive(Debug, PartialEq)]
pub enum AndOrOp {
    And,
    Or,
}
