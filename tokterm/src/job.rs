#![allow(unused_imports)]
#![allow(dead_code)]
#[allow(unused_imports, dead_code)]
use tracing::{info, error, debug, trace, warn};
use tokio::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;

use crate::common::*;

use super::stdio::*;
use super::reactor::*;

#[derive(Debug)]
pub struct Job
{
    pub id: u32,
    pub stdio: Stdio,
    pub stdin_tx: mpsc::Sender<Vec<u8>>,
    pub job_list_tx: mpsc::Sender<Pid>,
    pub job_list_rx: Arc<Mutex<mpsc::Receiver<Pid>>>,
}

impl Clone
for Job
{
    fn clone(&self) -> Job {
        Job {
            id: self.id,
            stdio: self.stdio.clone(),
            stdin_tx: self.stdin_tx.clone(),
            job_list_tx: self.job_list_tx.clone(),
            job_list_rx: self.job_list_rx.clone(),
        }
    }
}

impl Job
{
    pub fn new(id: u32, stdio: Stdio, stdin_tx: mpsc::Sender<Vec<u8>>) -> Job {
        let (job_list_tx, job_list_rx) = mpsc::channel(MAX_MPSC);
        Job {
            id,
            stdio,
            stdin_tx,
            job_list_tx,
            job_list_rx: Arc::new(Mutex::new(job_list_rx)),
        }
    }

    pub fn terminate(&self, reactor: &mut Reactor, exit_code: i32) {
        let mut rx = self.job_list_rx.lock().unwrap();
        while let Ok(pid) = rx.try_recv() {
            Reactor::close_process(reactor, pid, exit_code);
        }
        debug!("job terminated (id={})", self.id);
    }
}