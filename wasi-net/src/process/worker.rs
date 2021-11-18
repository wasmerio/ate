use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;
use std::fs::File;
use std::io::Write;

use bytes::BytesMut;

use super::*;
use crate::backend::MessageProcess;
use crate::backend::utils::*;

#[derive(Debug)]
pub struct Worker
{
    file: File,
    exited: bool,
    tx_stdout: mpsc::Sender<Vec<u8>>,
    tx_stderr: mpsc::Sender<Vec<u8>>,
    tx_exit: mpsc::Sender<ExitStatus>,
}

impl Worker
{
    pub(super) fn new(file: File) -> (Arc<Mutex<Worker>>, ChildStdin, ChildStdout, ChildStderr, mpsc::Receiver<ExitStatus>)
    {
        let (tx_stdout, rx_stdout) = mpsc::channel();
        let (tx_stderr, rx_stderr) = mpsc::channel();
        let (tx_exit, rx_exit) = mpsc::channel();

        let worker = Worker {
            file,
            exited: false,
            tx_stdout,
            tx_stderr,
            tx_exit,
        };
        let worker = Arc::new(Mutex::new(worker));

        let stdin = ChildStdin { worker: worker.clone() };
        let stdout = ChildStdout { rx: rx_stdout, buffer: BytesMut::new(), worker: worker.clone() };
        let stderr = ChildStderr { rx: rx_stderr, buffer: BytesMut::new(), worker: worker.clone() };

        (worker, stdin, stdout, stderr, rx_exit)
    }

    pub(super) fn work(&mut self) -> bool {
        if self.exited {
            return false;
        }
        if let Ok(res) = read_response(&mut self.file) {
            match res {
                MessageProcess::Exited(exitcode) => {
                    self.exited = true;
                    let _ = self.tx_exit.send(ExitStatus { code: Some(exitcode) });
                    return false;
                },
                MessageProcess::Stdout(data) => {
                    let _ = self.tx_stdout.send(data);
                },
                MessageProcess::Stderr(data) => {
                    let _ = self.tx_stderr.send(data);
                },
                _ => { }
            }
        }
        true
    }

    pub(super) fn send(&mut self, msg: MessageProcess) -> Result<()> {
        let mut submit = msg.serialize()?;
        submit += "\n";
        let _ = self.file.write_all(submit.as_bytes());
        Ok(())
    }
}