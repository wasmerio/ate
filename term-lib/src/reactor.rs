#![allow(unused_imports)]
#![allow(dead_code)]
use bytes::{Buf, BytesMut};
use std::collections::HashMap;
use std::num::NonZeroI32;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::sync::Mutex as AsyncMutex;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use super::api::*;
use super::bus::WasmBusThreadPool;
use super::common::*;
use super::environment::*;
use super::err::*;
use super::eval::*;
use super::fd::*;
use super::fs::*;
use super::job::*;
use super::poll::*;
use super::stdio::*;

#[derive(Debug)]
pub struct Reactor {
    pub(crate) system: System,
    pub(crate) pid_seed: Pid,
    pub(crate) pid: HashMap<Pid, Process>,
    pub(crate) job: HashMap<u32, Job>,
    pub(crate) current_job: Option<u32>,
}

impl Reactor {
    pub fn new() -> Reactor {
        Reactor {
            system: System::default(),
            pid_seed: 1,
            pid: HashMap::default(),
            job: HashMap::default(),
            current_job: None,
        }
    }

    pub fn get_process(&self, pid: Pid) -> Option<Process> {
        if let Some(process) = self.pid.get(&pid) {
            Some(process.clone())
        } else {
            None
        }
    }

    pub fn generate_pid(
        &mut self,
        thread_pool: Arc<WasmBusThreadPool>,
        forced_exit: Arc<AtomicI32>,
    ) -> Result<Pid, i32> {
        for _ in 0..10000 {
            let pid = self.pid_seed;
            self.pid_seed += 1;
            if self.pid.contains_key(&pid) == false {
                self.pid.insert(
                    pid,
                    Process {
                        system: self.system,
                        pid,
                        thread_pool,
                        forced_exit,
                    },
                );
                return Ok(pid);
            }
        }
        Err(ERR_EMFILE)
    }

    pub fn close_process(&mut self, pid: Pid, exit_code: i32) -> i32 {
        if let Some(process) = self.pid.remove(&pid) {
            info!("process closed (pid={})", pid);
            let exit_code = NonZeroI32::new(exit_code)
                .unwrap_or_else(|| NonZeroI32::new(ERR_ECONNABORTED).unwrap());
            process.terminate(exit_code);
        }
        ERR_OK
    }

    pub fn generate_job(
        &mut self,
        working_dir: String,
        env: Environment,
        root: UnionFileSystem,
    ) -> Result<(u32, Job), i32> {
        let mut job_seed = 1;
        for _ in 0..10000 {
            let id = job_seed;
            job_seed += 1;
            if self.job.contains_key(&id) == false {
                let job = Job::new(id, working_dir, env, root);
                self.job.insert(id, job.clone());
                return Ok((id, job));
            }
        }
        Err(ERR_EMFILE)
    }

    pub fn close_job(&mut self, job: Job, exit_code: i32) {
        let job_id = job.id;
        if self.current_job == Some(job_id) {
            self.current_job.take();
        }
        if let Some(job) = self.job.remove(&job_id) {
            job.terminate(self, exit_code);
            info!("job closed: id={}", job.id);
        } else {
            debug!("job already closed: id={}", job_id);
        }
    }

    pub fn get_job(&self, job_id: u32) -> Option<Job> {
        self.job.get(&job_id).map(|a| a.clone())
    }

    pub fn set_current_job(&mut self, job_id: u32) -> bool {
        if self.job.contains_key(&job_id) == false {
            return false;
        }
        self.current_job.replace(job_id);
        true
    }

    pub fn get_current_job(&self) -> Option<Job> {
        self.current_job
            .iter()
            .filter_map(|job| self.get_job(*job))
            .next()
    }
}
