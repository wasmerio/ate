use std::fmt;

#[derive(Debug)]
pub enum WebGlError
{
    IO(std::io::Error),
    CompileError(String),
    LinkError(String),
}

impl fmt::Display for WebGlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WebGlError::IO(err) => {
                fmt::Display::fmt(err, f)
            }
            WebGlError::CompileError(err) => {
                write!(f, "compile error - {}", err)
            }
            WebGlError::LinkError(err) => {
                write!(f, "link error - {}", err)
            }
        }
    }
}

impl std::error::Error for WebGlError {
}