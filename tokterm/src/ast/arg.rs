#[derive(Debug, PartialEq)]
pub enum Arg<'a> {
    Arg(&'a str),
    Backquote(Vec<Arg<'a>>),
}