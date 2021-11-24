use std::io::Write;
use std::io;

use super::*;
use crate::abi::*;
use crate::backend::process::*;

#[derive(Debug)]
pub struct ChildStdin {
    pub(super) task: Call,
}

impl ChildStdin {
    pub fn new(task: Call) -> ChildStdin {
        ChildStdin {
            task,
        }
    }
}

impl Write for ChildStdin {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        Ok(self.task.call(OutOfBand::DataStdin(buf.to_vec()))
            .invoke()
            .join()
            .wait()
            .map_err(|err| err.into_io_error())?)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}