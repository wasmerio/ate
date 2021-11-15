#![allow(unused_imports)]
#![allow(dead_code)]
use bytes::{Buf, BytesMut};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::sync::Mutex as AsyncMutex;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use super::common::*;
use super::environment::*;
use super::err::*;
use super::eval::*;
use super::fd::*;
use super::job::*;
use super::poll::*;
use super::pool::ThreadPool as Pool;
use super::stdio::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReceiverMode {
    Stream,
    Message(bool),
}

#[derive(Debug)]
pub struct ReactorPipeReceiver {
    pub(crate) rx: mpsc::Receiver<Vec<u8>>,
    pub(crate) buffer: BytesMut,
    pub(crate) mode: ReceiverMode,
}

#[derive(Debug)]
pub struct Reactor {
    pub(crate) fd_seed: usize,
    pub(crate) fd_pipe_tx: HashMap<RawFd, mpsc::Sender<Vec<u8>>>,
    pub(crate) fd_pipe_rx: HashMap<RawFd, Arc<Mutex<ReactorPipeReceiver>>>,

    pub(crate) pid_seed: Pid,
    pub(crate) pid: HashMap<Pid, Process>,

    pub(crate) job: HashMap<u32, Job>,
}

impl Reactor {
    pub fn new() -> Reactor {
        Reactor {
            pid_seed: 1,
            pid: HashMap::default(),
            fd_seed: 3,
            fd_pipe_rx: HashMap::default(),
            fd_pipe_tx: HashMap::default(),
            job: HashMap::default(),
        }
    }

    pub fn generate_fd_with_id(&mut self, fd: RawFd) -> Result<RawFd, i32> {
        if self.fd_pipe_rx.contains_key(&fd) == false && self.fd_pipe_tx.contains_key(&fd) == false
        {
            return Ok(fd);
        }
        Err(ERR_EMFILE)
    }

    pub fn generate_fd(&mut self) -> Result<RawFd, i32> {
        for _ in 0..10000 {
            let fd = self.fd_seed.into();
            self.fd_seed += 1;
            if let Ok(fd) = self.generate_fd_with_id(fd) {
                return Ok(fd);
            }
        }
        Err(ERR_EMFILE)
    }

    pub fn fd(&self, fd: RawFd) -> Fd {
        Fd::new(fd, self)
    }

    pub fn get_process(&self, pid: Pid) -> Option<Process> {
        if let Some(process) = self.pid.get(&pid) {
            Some(process.clone())
        } else {
            None
        }
    }

    pub fn generate_pid(&mut self) -> Result<(Pid, watch::Receiver<Option<i32>>), i32> {
        let (exit_tx, exit_rx) = watch::channel::<Option<i32>>(None);
        for _ in 0..10000 {
            let pid = self.pid_seed;
            self.pid_seed += 1;
            if self.pid.contains_key(&pid) == false {
                self.pid.insert(
                    pid,
                    Process {
                        pid,
                        exit_rx: exit_rx.clone(),
                        exit_tx: Arc::new(exit_tx),
                    },
                );
                return Ok((pid, exit_rx));
            }
        }
        Err(ERR_EMFILE)
    }

    pub fn bidirectional(
        &mut self,
        buffer_size_tx: usize,
        buffer_size_rx: usize,
        mode: ReceiverMode,
    ) -> Result<(RawFd, mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>), i32> {
        let fd = self.generate_fd()?;
        let (tx_send, rx_send) = mpsc::channel(buffer_size_tx);
        let (tx_recv, rx_recv) = mpsc::channel(buffer_size_rx);
        self.fd_pipe_rx.insert(
            fd,
            Arc::new(Mutex::new(ReactorPipeReceiver {
                rx: rx_send,
                buffer: BytesMut::new(),
                mode,
            })),
        );
        self.fd_pipe_tx.insert(fd, tx_recv);
        Ok((fd, tx_send, rx_recv))
    }

    pub fn pipe_out(&mut self) -> Result<(RawFd, mpsc::Receiver<Vec<u8>>), i32> {
        let fd = self.generate_fd()?;
        let (tx, rx) = mpsc::channel(MAX_MPSC);
        self.fd_pipe_tx.insert(fd, tx);
        Ok((fd, rx))
    }

    pub fn dup(&mut self, fd: &RawFd) -> Result<RawFd, i32> {
        let new_fd = self.generate_fd()?;

        let tx = if let Some(tx) = self.fd_pipe_tx.get(fd) {
            Some(tx.clone())
        } else {
            None
        };
        if let Some(tx) = tx {
            self.fd_pipe_tx.insert(new_fd, tx);
        }

        let rx = if let Some(rx) = self.fd_pipe_rx.get(fd) {
            Some(rx.clone())
        } else {
            None
        };
        if let Some(rx) = rx {
            self.fd_pipe_rx.insert(new_fd, rx);
        }

        Ok(new_fd)
    }

    pub fn pipe_in(&mut self, mode: ReceiverMode) -> Result<(RawFd, mpsc::Sender<Vec<u8>>), i32> {
        let fd = self.generate_fd()?;
        self.pipe_in_with_fd(fd, mode)
    }

    pub fn pipe_in_with_fd(
        &mut self,
        fd: RawFd,
        mode: ReceiverMode,
    ) -> Result<(RawFd, mpsc::Sender<Vec<u8>>), i32> {
        let (tx, rx) = mpsc::channel(MAX_MPSC);
        let rx = ReactorPipeReceiver {
            rx,
            buffer: BytesMut::new(),
            mode,
        };
        self.fd_pipe_rx.insert(fd, Arc::new(Mutex::new(rx)));
        Ok((fd, tx))
    }

    pub fn pipe(&mut self, mode: ReceiverMode) -> Result<(RawFd, RawFd), i32> {
        let (fd_rx, tx2) = self.pipe_in(mode)?;
        let (fd_tx, mut rx2) = self.pipe_out()?;
        wasm_bindgen_futures::spawn_local(async move {
            while let Some(data) = rx2.recv().await {
                let _ = tx2.send(data).await;
            }
        });
        Ok((fd_tx, fd_rx))
    }

    pub fn remove_pipe(&mut self, fd: RawFd) -> i32 {
        self.fd_pipe_rx.remove(&fd);
        self.fd_pipe_tx.remove(&fd);
        ERR_OK
    }

    pub fn close_process(&mut self, pid: Pid, exit_code: i32) -> i32 {
        if let Some(mut process) = self.pid.remove(&pid) {
            debug!("process closed (pid={})", pid);
            process.terminate(exit_code);
        }
        ERR_OK
    }

    pub fn generate_job(
        &mut self,
        stdio: Stdio,
        stdin_tx: mpsc::Sender<Vec<u8>>,
    ) -> Result<(u32, Job), i32> {
        let mut job_seed = 1;
        for _ in 0..10000 {
            let id = job_seed;
            job_seed += 1;
            if self.job.contains_key(&id) == false {
                let job = Job::new(id, stdio, stdin_tx);
                self.job.insert(id, job.clone());
                return Ok((id, job));
            }
        }
        Err(ERR_EMFILE)
    }

    pub fn close_job(&mut self, job: Job, exit_code: i32) {
        let job_id = job.id;
        if let Some(job) = self.job.remove(&job_id) {
            job.terminate(self, exit_code);
            info!("job closed: id={}", job.id);
        } else {
            debug!("job already closed: id={}", job_id);
        }
    }
}
