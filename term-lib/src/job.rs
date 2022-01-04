#![allow(unused_imports)]
#![allow(dead_code)]
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::common::*;

use super::environment::*;
use super::fd::*;
use super::fs::*;
use super::pipe::*;
use super::reactor::*;
use super::stdio::*;

#[derive(Debug)]
pub struct Job {
    pub id: u32,
    pub stdin: Fd,
    pub stdin_tx: mpsc::Sender<FdMsg>,
    pub job_list_tx: mpsc::Sender<Pid>,
    pub job_list_rx: Arc<Mutex<mpsc::Receiver<Pid>>>,
    pub working_dir: String,
    pub env: Arc<Environment>,
    pub root: UnionFileSystem,
}

impl Clone for Job {
    fn clone(&self) -> Job {
        Job {
            id: self.id,
            stdin: self.stdin.clone(),
            stdin_tx: self.stdin_tx.clone(),
            job_list_tx: self.job_list_tx.clone(),
            job_list_rx: self.job_list_rx.clone(),
            working_dir: self.working_dir.clone(),
            env: self.env.clone(),
            root: self.root.clone(),
        }
    }
}

impl Job {
    pub fn new(id: u32, working_dir: String, env: Environment, root: UnionFileSystem) -> Job {
        let (stdin, stdin_tx) = pipe_in(ReceiverMode::Stream, FdFlag::Stdin(true));
        let (job_list_tx, job_list_rx) = mpsc::channel(MAX_MPSC);
        Job {
            id,
            stdin,
            stdin_tx,
            job_list_tx,
            job_list_rx: Arc::new(Mutex::new(job_list_rx)),
            working_dir,
            env: Arc::new(env),
            root,
        }
    }

    pub fn terminate(&self, reactor: &mut Reactor, exit_code: i32) {
        self.stdin.forced_exit(exit_code);
        let mut rx = self.job_list_rx.lock().unwrap();
        while let Ok(pid) = rx.try_recv() {
            Reactor::close_process(reactor, pid, exit_code);
        }
        debug!("job terminated (id={})", self.id);
    }
}
