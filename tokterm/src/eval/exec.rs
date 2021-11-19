#![allow(dead_code)]
#![allow(unused)]
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::watch;
use tokio::sync::RwLock;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use wasmer::{Instance, Module, Store};
use wasmer_vfs::FileSystem;
use wasmer_vfs::FsError;
use wasmer_wasi::Stdin;
use wasmer_wasi::{Stdout, WasiError, WasiState};
use web_sys::{console, HtmlElement, HtmlInputElement, Worker};
use web_sys::{Request, RequestInit, RequestMode, Response};

use super::*;

use crate::bin::*;
use crate::builtins::*;
use crate::common::*;
use crate::environment::*;
use crate::err;
use crate::err::*;
use crate::fs::*;
use crate::job::*;
use crate::pipe::*;
use crate::poll::*;
use crate::pool::*;
use crate::reactor::*;
use crate::state::*;
use crate::stdio::*;

pub enum ExecResponse {
    Immediate(i32),
    Process(Process),
}

pub async fn exec(
    ctx: &mut EvalContext,
    builtins: &Builtins,
    cmd: &String,
    args: &Vec<String>,
    env_vars: &Vec<String>,
    show_result: &mut bool,
    mut stdio: Stdio,
    redirect: &Vec<Redirect>,
) -> Result<ExecResponse, i32> {
    // Make an error message function
    let mut early_stderr = stdio.stderr.clone();
    let on_early_exit = |msg: Option<String>, res: ExecResponse| async move {
        *show_result = true;
        if let Some(msg) = msg {
            let _ = early_stderr.write(msg.as_bytes()).await;
        }
        Ok(res)
    };

    // If there is a built in then use it
    if let Some(builtin) = builtins.get(cmd) {
        return on_early_exit(
            None,
            match builtin(args, ctx, stdio).await {
                Ok(a) => a,
                Err(err) => ExecResponse::Immediate(err),
            },
        )
        .await;
    }

    // Grab the private file system for this binary (if the binary changes the private
    // file system will also change)
    let (data, fs_private) = match load_bin(ctx, cmd, &mut stdio).await {
        Some(a) => a,
        None => {
            return on_early_exit(None, ExecResponse::Immediate(err::ERR_ENOENT)).await;
        }
    };

    // Create the filesystem
    let fs = {
        let root = ctx.root.clone();
        let stdio = stdio.clone();
        let tok = TokeraSocket::new(
            &ctx.reactor,
            ctx.exec_factory.clone(),
            stdio.stdin.clone(),
            stdio.stdout.clone(),
            stdio.stderr.clone(),
        );

        let mut union = UnionFileSystem::new();
        union.mount("root", Path::new("/"), Box::new(root));
        union.mount(
            "proc",
            Path::new("/dev"),
            Box::new(ProcFileSystem::new(stdio, tok)),
        );
        union.mount("tmp", Path::new("/tmp"), Box::new(TmpFileSystem::default()));
        union.mount("private", Path::new("/.private"), Box::new(fs_private));
        Box::new(union)
    };

    // Perfform all the redirects
    for redirect in redirect.iter() {
        // Attempt to open the file
        let file = fs
            .new_open_options()
            .create(redirect.op.write())
            .create_new(redirect.op.write() && redirect.op.append() == false)
            .truncate(redirect.op.write() && redirect.op.append() == false)
            .append(redirect.op.append())
            .read(redirect.op.read())
            .write(redirect.op.write())
            .open(redirect.filename.clone());
        match file {
            Ok(mut file) => {
                // Open a new file description
                let (tx, mut rx) = {
                    let (fd, tx, rx) = bidirectional_with_defaults();

                    // We now connect the newly opened file descriptor with the read file
                    match redirect.fd {
                        -1 => {
                            if redirect.op.read() {
                                stdio.stdin = fd.clone();
                            }
                            if redirect.op.write() {
                                stdio.stdout = fd;
                            }
                        }
                        0 => stdio.stdin = fd,
                        1 => stdio.stdout = fd,
                        2 => stdio.stderr = fd,
                        _ => {
                            return on_early_exit(Some(format!("redirecting non-standard file descriptors is not yet supported")), ExecResponse::Immediate(err::ERR_EINVAL)).await;
                        }
                    };

                    // Now we need to hook up the receiver and sender
                    (tx, rx)
                };

                // Now hook up the sender and receiver
                let is_read = redirect.op.read();
                let is_write = redirect.op.write();
                ctx.pool.spawn_blocking(move || {
                    if is_read {
                        let mut buf = [0u8; 4096];
                        while let Ok(read) = file.read(&mut buf) {
                            let _ = tx.blocking_send((&buf[0..read]).to_vec());
                        }
                    }
                    if is_write {
                        wasm_bindgen_futures::spawn_local(async move {
                            while let Some(data) = rx.recv().await {
                                let _ = file.write_all(&data[..]);
                            }
                        });
                    }
                });
            }
            Err(err) => {
                return on_early_exit(
                    Some(format!("failed to open the redirected file")),
                    ExecResponse::Immediate(match err {
                        FsError::EntityNotFound => ERR_ENOENT,
                        _ => ERR_EIO,
                    }),
                )
                .await;
            }
        }
    }

    // Generate a PID for this process
    let (pid, mut exit_rx, exit_tx, process) = {
        let mut guard = ctx.reactor.write().await;
        let (pid, exit_rx) = guard.generate_pid(ctx.pool.clone())?;
        let process = match guard.get_process(pid) {
            Some(a) => a,
            None => {
                return Err(ERR_ESRCH);
            }
        };
        let exit_tx = process.exit_tx.clone();
        (pid, exit_rx, exit_tx, process)
    };
    debug!("process created (pid={})", pid);

    // Spawn the process on a background thread
    let mut tty = ctx.stdio.stderr.clone();
    let reactor = ctx.reactor.clone();
    let cmd = cmd.clone();
    let args = args.clone();
    let path = ctx.path.clone();
    let process2 = process.clone();
    let preopen = ctx.pre_open.clone();
    ctx.pool.spawn_blocking(move || {
        // Compile the module (which)
        let _ = tty.blocking_write("Compiling...".as_bytes());
        let store = Store::default();
        let module = match Module::new(&store, &data[..]) {
            Ok(a) => a,
            Err(err) => {
                tty.blocking_write_clear_line();
                let _ = tty.blocking_write(format!("compile-error: {}\n", err).as_bytes());
                process.terminate(ERR_ENOEXEC);
                return;
            }
        };
        tty.blocking_write_clear_line();
        info!(
            "compiled {}",
            module.name().unwrap_or_else(|| "unknown module")
        );

        // Build the list of arguments
        let args = args.iter().skip(1).map(|a| a.as_str()).collect::<Vec<_>>();

        // Create the `WasiEnv`.
        let mut wasi_env = WasiState::new(cmd.as_str());
        let mut wasi_env = wasi_env
            .args(&args)
            .stdin(Box::new(stdio.stdin.clone()))
            .stdout(Box::new(stdio.stdout.clone()))
            .stderr(Box::new(stdio.stderr.clone()));

        // Add the extra pre-opens
        if preopen.len() > 0 {
            for pre_open in preopen {
                if wasi_env.preopen_dir(Path::new(pre_open.as_str())).is_ok() == false {
                    tty.blocking_write_clear_line();
                    let _ = tty.blocking_write(format!("pre-open error (path={})\n", pre_open).as_bytes());
                    process.terminate(ERR_ENOEXEC);
                    return;
                }
            }

        // Or we default and open the current directory
        } else {
            wasi_env
                .preopen_dir(Path::new("/"))
                .unwrap()
                .map_dir(".", Path::new(path.as_str()));
        }

        // Finish off the WasiEnv
        let mut wasi_env = match wasi_env
            .set_fs(fs)
            .finalize()
        {
            Ok(a) => a,
            Err(err) => {
                tty.blocking_write_clear_line();
                let _ = tty.blocking_write(format!("exec error: {}\n", err.to_string()).as_bytes());
                process.terminate(ERR_ENOEXEC);
                return;
            }
        };

        // Hook up the terminate event so that if its triggered the environment properly
        // kills itself (on the next syscall)
        process.set_env(wasi_env.clone());

        // Generate an `ImportObject`.
        let import_object = wasi_env.import_object(&module).unwrap();

        // Let's instantiate the module with the imports.
        let instance = Instance::new(&module, &import_object).unwrap();

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
                Ok(a) => err::ERR_OK,
                Err(e) => match e.downcast::<WasiError>() {
                    Ok(WasiError::Exit(code)) => code as i32,
                    Ok(WasiError::UnknownWasiVersion) => {
                        let _ = stdio.stderr.blocking_write(
                            &format!("exec-failed: unknown wasi version\n").as_bytes()[..],
                        );
                        err::ERR_ENOEXEC
                    }
                    Err(err) => {
                        err::ERR_PANIC
                    }
                },
            }
        } else {
            let _ = stdio
                .stderr
                .blocking_write(&format!("exec-failed: missing _start function\n").as_bytes()[..]);
            err::ERR_ENOEXEC
        };
        debug!("exited with code {}", ret);
        process.terminate(ret);
    });

    Ok(ExecResponse::Process(process2))
}

pub async fn waitpid(reactor: &Arc<RwLock<Reactor>>, pid: Pid) -> i32 {
    let process = {
        let reactor = reactor.read().await;
        reactor.get_process(pid)
    };
    if let Some(mut process) = process {
        process.wait_for_exit().await
    } else {
        ERR_ESRCH
    }
}
