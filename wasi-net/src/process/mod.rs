mod child;
mod child_stderr;
mod child_stdin;
mod child_stdout;
mod command;
mod exit_status;
mod output;
mod stdio;
mod worker;

use worker::Worker;

pub use child::Child;
pub use child_stderr::ChildStderr;
pub use child_stdin::ChildStdin;
pub use child_stdout::ChildStdout;
pub use command::Command;
pub use exit_status::ExitStatus;
pub use output::Output;
pub use stdio::*;

pub use std::io::Error;
pub use std::io::ErrorKind;
pub use std::io::Result;
