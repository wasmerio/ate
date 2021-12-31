use std::io;
use std::io::Write;

use super::*;
use crate::api;

#[derive(Debug)]
pub struct ChildStdin {
    pub(super) context: api::ProcessClient,
}

impl ChildStdin {
    pub fn new(context: api::ProcessClient) -> ChildStdin {
        ChildStdin { context }
    }
}

impl Write for ChildStdin {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        Ok(self
            .context
            .blocking_stdin(buf.to_vec())
            .map_err(|err| err.into_io_error())?)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(self
            .context
            .blocking_flush()
            .map_err(|err| err.into_io_error())?)
    }
}
