use bytes::*;
use std::io::Read;
use std::sync::mpsc;

use super::*;

#[derive(Debug)]
pub struct ChildStdout {
    pub(super) rx: mpsc::Receiver<Vec<u8>>,
    pub(super) buffer: BytesMut,
}

impl ChildStdout {
    pub fn new() -> (ChildStdout, mpsc::Sender<Vec<u8>>) {
        let (tx_stdout, rx_stdout) = mpsc::channel();
        (
            ChildStdout {
                rx: rx_stdout,
                buffer: BytesMut::new(),
            },
            tx_stdout,
        )
    }
}

impl Read for ChildStdout {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        loop {
            if self.buffer.has_remaining() {
                let max = self.buffer.remaining().min(buf.len());
                buf[0..max].copy_from_slice(&self.buffer[..max]);
                self.buffer.advance(max);
                return Ok(max);
            } else {
                match self.rx.recv() {
                    Ok(data) => {
                        self.buffer.extend_from_slice(&data[..]);
                    }
                    Err(mpsc::RecvError) => {
                        return Ok(0usize);
                    }
                }
            }
        }
    }
}
