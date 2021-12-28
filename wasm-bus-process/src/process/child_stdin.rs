use std::io;
use std::io::Write;

use super::*;
use crate::api::Process;

#[derive(Debug)]
pub struct ChildStdin {
    pub(super) context: Process,
}

impl ChildStdin {
    pub fn new(context: Process) -> ChildStdin {
        ChildStdin { context }
    }
}

impl Write for ChildStdin {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        Ok(self
            .context
            .stdin(buf.to_vec())
            .join()
            .wait()
            .map_err(|err| err.into_io_error())?)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(self
            .context
            .flush()
            .join()
            .wait()
            .map_err(|err| err.into_io_error())?)
    }
}
