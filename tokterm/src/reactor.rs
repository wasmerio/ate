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

#[derive(Debug)]
pub struct Reactor {
    pub(crate) pid_seed: Pid,
    pub(crate) pid: HashMap<Pid, Process>,
    pub(crate) job: HashMap<u32, Job>,
}

impl Reactor {
    pub fn new() -> Reactor {
        Reactor {
            pid_seed: 1,
            pid: HashMap::default(),
            job: HashMap::default(),
        }
    }

    pub fn get_process(&self, pid: Pid) -> Option<Process> {
        if let Some(process) = self.pid.get(&pid) {
            Some(process.clone())
        } else {
            None
        }
    }

    pub fn generate_pid(&mut self, pool: Pool) -> Result<(Pid, watch::Receiver<Option<i32>>), i32> {
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
                        pool,
                        env: Arc::new(Mutex::new(None))
                    },
                );
                return Ok((pid, exit_rx));
            }
        }
        Err(ERR_EMFILE)
    }

    pub fn close_process(&mut self, pid: Pid, exit_code: i32) -> i32 {
        if let Some(process) = self.pid.remove(&pid) {
            info!("process closed (pid={})", pid);
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
