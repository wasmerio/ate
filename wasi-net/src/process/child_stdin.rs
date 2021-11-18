use std::io::Write;
use std::sync::Arc;
use std::sync::Mutex;

use super::*;
use crate::backend::MessageProcess;

#[derive(Debug)]
pub struct ChildStdin {
    pub(super) worker: Arc<Mutex<Worker>>,
}

impl Write for ChildStdin {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let msg = MessageProcess::Stdin(buf.to_vec());
        self.worker.lock().unwrap().send(msg)?;
        return Ok(buf.len());
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}
