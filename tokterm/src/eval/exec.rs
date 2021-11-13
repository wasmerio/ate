#![allow(dead_code)]
#![allow(unused)]
#[allow(unused_imports, dead_code)]
use tracing::{info, error, debug, trace, warn};
use wasmer::{Instance, Module, Store};
use wasmer_wasi::{Stdout, WasiState, WasiError, WasiProxy};
use wasmer_wasi::{Stdin};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{console, HtmlElement, HtmlInputElement, Worker};
use web_sys::{Request, RequestInit, RequestMode, Response};
use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::RwLock;
use std::sync::Arc;
use std::sync::Mutex;
use std::path::{Path, PathBuf};
use wasmer_wasi::vfs::FileSystem;
use bytes::Bytes;
use std::io::Read;

use super::*;

use crate::wasi::*;
use crate::state::*;
use crate::common::*;
use crate::builtins::*;
use crate::stdio::*;
use crate::reactor::*;
use crate::poll::*;
use crate::err::*;
use crate::err;
use crate::pool::*;
use crate::environment::*;
use crate::job::*;
use crate::bin::*;
use crate::fs::*;

pub enum ExecResponse
{
    Immediate(i32),
    Process(Process),
}

pub async fn exec
(
    ctx: &mut EvalContext,
    builtins: &Builtins,
    cmd: &String,
    args: &Vec<String>,
    env_vars: &Vec<String>,
    show_result: &mut bool,
    mut stdio: Stdio,
) -> Result<ExecResponse, i32>
{
    // If there is a built in then use it
    if let Some(builtin) = builtins.get(cmd) {
        *show_result = true;
        let ret = builtin(args, ctx, stdio).await;
        return Ok(ExecResponse::Immediate(ret));
    }

    // Grab the private file system for this binary (if the binary changes the private
    // file system will also change)
    let (data, fs_private) = match load_bin(ctx, cmd, &mut stdio).await {
        Some(a) => a,
        None => {
            *show_result = true;
            return Ok(ExecResponse::Immediate(err::ERR_ENOENT));
        }
    };

    // Generate a PID for this process
    let (pid, exit_rx, exit_tx, process) = {
        let mut guard = ctx.reactor.write().await;
        let (pid, exit_rx) = guard.generate_pid()?;
        let process = match guard.get_process(pid) {
            Some(a) => a,
            None => { return Err(ERR_ESRCH); }
        };
        let exit_tx = process.exit_tx.clone();
        (pid, exit_rx, exit_tx, process)
    };
    debug!("process created (pid={})", pid);

    // Get the stdout and stderr
    let stdin = stdio.stdin.clone();
    let stdout = stdio.stdout.clone();
    let stderr = stdio.stderr.clone();

    // Spawn the process on a background thread
    let reactor = ctx.reactor.clone();
    let cmd = cmd.clone();
    let args = args.clone();
    let path = ctx.path.clone();
    ctx.pool.spawn_blocking(move ||
    {
        // Compile the module (which)
        let store = Store::default();
        let module = match Module::new(&store, &data[..]) {
            Ok(a) => a,
            Err(err) => {
                let _ = stdio.stderr.blocking_write(&format!("compile-error: {}\n", err).as_bytes()[..]);
                exit_tx.send(Some(ERR_ENOEXEC));
                return;
            }
        };
        info!("compiled {}", module.name().unwrap_or_else(|| "unknown module"));

        // Build the list of arguments
        let args = args.iter()
            .skip(1)
            .map(|a| a.as_str())
            .collect::<Vec<_>>();

        // Create the WasiProxy
        let wasi_proxy = WasiTerm::new(&reactor, exit_rx);

        // Create the filesystem
        let fs = {
            let root = stdio.root.clone();
            let stdio = Stdio {
                stdin: stdin.clone(),
                stdout: stdout.clone(),
                stderr: stderr.clone(),
                tty: stdio.tty.clone(),
                tok: stdio.tok.clone(),
                root: root.clone(),
            };

            let mut union = UnionFileSystem::new();
            union.mount("root", Path::new("/"), Box::new(root));
            union.mount("proc", Path::new("/dev"), Box::new(ProcFileSystem::new(stdio)));
            union.mount("tmp", Path::new("/tmp"), Box::new(TmpFileSystem::default()));
            union.mount("private", Path::new("/.private"), Box::new(fs_private));
            Box::new(union)
        };

        // Create the `WasiEnv`.
        let mut wasi_env = WasiState::new(cmd.as_str())
            .args(&args)
            .stdin(Box::new(stdin))
            .stdout(Box::new(stdout))
            .stderr(Box::new(stderr))
            .syscall_proxy(Box::new(wasi_proxy))
            .preopen_dir(Path::new("/")).unwrap()
            .set_fs(fs)
            .finalize()
            .unwrap();

        // Generate an `ImportObject`.
        let import_object = wasi_env.import_object(&module)
            .unwrap();

        // Let's instantiate the module with the imports.
        let instance = Instance::new(&module, &import_object)
            .unwrap();

        // Let's call the `_start` function, which is our `main` function in Rust.
        let start = instance
            .exports
            .get_native_function::<(), ()>("_start")
            .ok();

        // Set the panic handler
        
        // If there is a start function
        debug!("called main() on {}", cmd);
        let ret = if let Some(start) = start {
            match start.call() {
                Ok(a) => {
                    err::ERR_OK
                },
                Err(e) => match e.downcast::<WasiError>() {
                    Ok(WasiError::Exit(code)) => {
                        code as i32
                    },
                    Ok(WasiError::UnknownWasiVersion) => {
                        let _ = stdio.stderr.blocking_write(&format!("exec-failed: unknown wasi version\n").as_bytes()[..]);
                        err::ERR_ENOEXEC            
                    },
                    Err(err) => {
                        let _ = stdio.stderr.blocking_write(&format!("exec error: {}\n", err).as_bytes()[..]);
                        err::ERR_PANIC
                    }
                }
            }
        } else {
            let _ = stdio.stderr.blocking_write(&format!("exec-failed: missing _start function\n").as_bytes()[..]);
            err::ERR_ENOEXEC
        };
        debug!("exited with code {}", ret);
        exit_tx.send(Some(ret));
    });
    
    Ok(ExecResponse::Process(process))
}

pub async fn waitpid(reactor: &Arc<RwLock<Reactor>>, pid: Pid) -> i32
{
    let process = {
        let reactor = reactor.read().await;
        reactor.get_process(pid)
    };
    if let Some(mut process) = process  {
        process.wait_for_exit().await
    } else {
        ERR_ESRCH
    }
}