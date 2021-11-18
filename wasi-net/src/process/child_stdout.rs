use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;
use std::io::Read;
use bytes::*;

use super::*;

#[derive(Debug)]
pub struct ChildStdout
{
    pub(super) rx: mpsc::Receiver<Vec<u8>>,
    pub(super) buffer: BytesMut,
    pub(super) worker: Arc<Mutex<Worker>>,
}

impl Read
for ChildStdout
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>
    {
        loop {
            if self.buffer.has_remaining() {
                let max = self.buffer.remaining().min(buf.len());
                buf[0..max].copy_from_slice(&self.buffer[..max]);
                self.buffer.advance(max);
                return Ok(max);
            } else {
                match self.rx.try_recv() {
                    Ok(data) => {
                        self.buffer.extend_from_slice(&data[..]);
                    },
                    Err(mpsc::TryRecvError::Disconnected) => {
                        return Ok(0usize);
                    },
                    Err(mpsc::TryRecvError::Empty) => {
                        let mut worker = self.worker.lock().unwrap();
                        if let Ok(data) = self.rx.try_recv() {
                            self.buffer.extend_from_slice(&data[..]);
                            continue;
                        }
                        if worker.work() == false {
                            return Ok(0usize);
                        }
                    }
                }
            }
        }
    }
}