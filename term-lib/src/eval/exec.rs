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

use super::*;

use crate::api::*;
use crate::bin_factory::*;
use crate::builtins::*;
use crate::bus::*;
use crate::common::*;
use crate::environment::*;
use crate::err;
use crate::err::*;
use crate::fs::*;
use crate::job::*;
use crate::pipe::*;
use crate::poll::*;
use crate::reactor::*;
use crate::state::*;
use crate::stdio::*;
use crate::wasmer::{ChainableNamedResolver, Instance, Module, Store};
use crate::wasmer_vfs::FileSystem;
use crate::wasmer_vfs::FsError;
use crate::wasmer_wasi::Stdin;
use crate::wasmer_wasi::{Stdout, WasiError, WasiState};

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

        let mut union = UnionFileSystem::new();
        union.mount("root", Path::new("/"), Box::new(root));
        union.mount(
            "proc",
            Path::new("/dev"),
            Box::new(ProcFileSystem::new(stdio)),
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

                // Now hook up the sender and receiver on a shared task
                let system = ctx.system;
                let is_read = redirect.op.read();
                let is_write = redirect.op.write();
                system.fork_shared(move || async move {
                    if is_read {
                        let mut buf = [0u8; 4096];
                        while let Ok(read) = file.read(&mut buf) {
                            let _ = tx.send((&buf[0..read]).to_vec()).await;
                        }
                    }
                    if is_write {
                        while let Some(data) = rx.recv().await {
                            let _ = file.write_all(&data[..]);
                        }
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
        let (pid, exit_rx) = guard.generate_pid()?;
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

    // Create the process factory that used by this process to create sub-processes
    let sub_process_factory = ProcessExecFactory::new(
        ctx.reactor.clone(),
        ctx.exec_factory.clone(),
        stdio.stdin.downgrade(),
        stdio.stdout.downgrade(),
        stdio.stderr.downgrade(),
    );

    // The BUS pool is what gives this WASM process its syscall and operation system
    // functions and services
    let bus_pool = WasmBusThreadPool::new(sub_process_factory);

    // Spawn the process on a background thread
    let mut tty = ctx.stdio.stderr.clone();
    let reactor = ctx.reactor.clone();
    let cmd = cmd.clone();
    let args = args.clone();
    let path = ctx.path.clone();
    let process2 = process.clone();
    let preopen = ctx.pre_open.clone();
    ctx.system.fork_dedicated(move |mut thread_local|
        async move {
            let mut thread_local = thread_local.borrow_mut();

            // Compile the module
            let _ = tty.write("Compiling...".as_bytes()).await;
            let store = &mut thread_local.store;
            let module = match Module::new(store, &data[..]) {
                Ok(a) => a,
                Err(err) => {
                    tty.write_clear_line().await;
                    let _ = tty.write(format!("compile-error: {}\n", err).as_bytes()).await;
                    process.terminate(ERR_ENOEXEC);
                    return;
                }
            };
            tty.write_clear_line().await;
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
                        tty.write_clear_line().await;
                        let _ = tty
                            .write(format!("pre-open error (path={})\n", pre_open).as_bytes()).await;
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

            // Add the tick callback that will invoke the WASM bus background
            // operations on the current thread
            {
                let bus_pool = Arc::clone(&bus_pool);
                wasi_env.on_yield(move |thread| {
                    if let Some(exit) = *exit_rx.borrow() {
                        thread.terminate(exit as u32);
                    }
                    let thread = bus_pool.get_or_create(thread);
                    unsafe {
                        crate::bus::syscalls::raw::wasm_bus_tick(&thread);
                    }
                });
            }

            // Finish off the WasiEnv
            let mut wasi_env = match wasi_env.set_fs(fs).finalize() {
                Ok(a) => a,
                Err(err) => {
                    drop(module);
                    tty.write_clear_line().await;
                    let _ = tty.write(format!("exec error: {}\n", err.to_string()).as_bytes()).await;
                    process.terminate(ERR_ENOEXEC);
                    return;
                }
            };

            // List all the exports
            for ns in module.exports() {
                trace!("module::export - {}", ns.name());
            }
            for ns in module.imports() {
                trace!("module::import - {}::{}", ns.module(), ns.name());
            }

            // Create the WASI thread
            let mut wasi_thread = wasi_env.new_thread();

            // Generate an `ImportObject`.
            let wasi_import = wasi_thread.import_object(&module).unwrap();
            let mut wasm_bus_import = bus_pool.get_or_create(&wasi_thread);
            let wasm_bus_import = wasm_bus_import.import_object(&module);
            let import = wasi_import.chain_front(wasm_bus_import);

            // Let's instantiate the module with the imports.
            let instance = Instance::new(&module, &import).unwrap();

            // Let's call the `_start` function, which is our `main` function in Rust.
            let start = instance
                .exports
                .get_native_function::<(), ()>("_start")
                .ok();

            // If there is a start function
            debug!("called main() on {}", cmd);
            let ret = if let Some(start) = start {
                match start.call() {
                    Ok(a) => err::ERR_OK,
                    Err(e) => match e.downcast::<WasiError>() {
                        Ok(WasiError::Exit(code)) => code as i32,
                        Ok(WasiError::UnknownWasiVersion) => {
                            let _ = stdio.stderr.write(
                                &format!("exec-failed: unknown wasi version\n").as_bytes()[..],
                            ).await;
                            err::ERR_ENOEXEC
                        }
                        Err(err) => err::ERR_PANIC,
                    },
                }
            } else {
                let _ = stdio
                    .stderr
                    .write(&format!("exec-failed: missing _start function\n").as_bytes()[..]).await;
                err::ERR_ENOEXEC
            };
            debug!("exited with code {}", ret);
            process.terminate(ret);
        }
    );

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
