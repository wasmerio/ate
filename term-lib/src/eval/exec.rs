#![allow(dead_code)]
#![allow(unused)]
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sha2::digest::generic_array::sequence::Lengthen;
use std::io::Read;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
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
#[cfg(feature = "wasmer-compiler-cranelift")]
use crate::wasmer_compiler_cranelift::Cranelift;
#[cfg(feature = "wasmer-compiler-llvm")]
use crate::wasmer_compiler_llvm::LLVM;
#[cfg(feature = "wasmer-compiler-singlepass")]
use crate::wasmer_compiler_singlepass::Singlepass;
use crate::wasmer_vfs::FileSystem;
use crate::wasmer_vfs::FsError;
use crate::wasmer_wasi::Stdin;
use crate::wasmer_wasi::{Stdout, WasiError, WasiState};
#[cfg(feature = "wasmer-compiler")]
use {crate::wasmer::Universal, crate::wasmer_compiler::CompilerConfig};

pub enum ExecResponse {
    Immediate(EvalContext, u32),
    OrphanedImmediate(u32),
    Process(Process, AsyncResult<(EvalContext, u32)>),
}

pub async fn exec(
    ctx: EvalContext,
    builtins: &Builtins,
    cmd: &String,
    args: &Vec<String>,
    env_vars: &Vec<String>,
    show_result: &mut bool,
    mut stdio: Stdio,
    redirect: &Vec<Redirect>,
) -> Result<ExecResponse, u32> {
    // If there is a built in then use it
    if let Some(builtin) = builtins.get(cmd) {
        *show_result = true;
        return Ok(builtin(&args, ctx, stdio).await);
    }

    let (process, process_result, _) =
        exec_process(ctx, cmd, args, env_vars, show_result, stdio, redirect).await?;

    Ok(ExecResponse::Process(process, process_result))
}

pub async fn exec_process(
    mut ctx: EvalContext,
    cmd: &String,
    args: &Vec<String>,
    env_vars: &Vec<String>,
    show_result: &mut bool,
    mut stdio: Stdio,
    redirect: &Vec<Redirect>,
) -> Result<(Process, AsyncResult<(EvalContext, u32)>, Arc<WasmBusThreadPool>), u32> {
    // Make an error message function
    let mut early_stderr = stdio.stderr.clone();
    let on_early_exit = |msg: Option<String>, err: u32| async move {
        *show_result = true;
        if let Some(msg) = msg {
            let _ = early_stderr.write(msg.as_bytes()).await;
        }
        Err(err)
    };

    // Grab the private file system for this binary (if the binary changes the private
    // file system will also change)
    let mut preopen = ctx.pre_open.clone();
    let mut chroot = ctx.chroot;
    let (data_hash, data, mut fs_private) = match load_bin(&ctx, cmd, &mut stdio).await {
        Some(a) => {
            if a.chroot {
                chroot = true;
            }
            preopen.extend(a.mappings.into_iter());

            (a.hash, a.data, a.fs)
        }
        None => {
            return on_early_exit(None, err::ERR_ENOENT).await;
        }
    };

    // If compile caching is enabled the load the module
    #[cfg(feature = "cached_compiling")]
    let mut module = { ctx.bins.get_compiled_module(&data_hash).await };

    // We listen for any forced exits using this channel
    let caller_ctx = WasmCallerContext::default();
    fs_private.set_ctx(&caller_ctx);

    // Create the filesystem
    let (fs, union) = {
        let root = ctx.root.clone();
        let stdio = stdio.clone();

        let mut union = root.clone();
        union.mount(
            "proc",
            "/dev",
            true,
            Box::new(ProcFileSystem::new(stdio)),
        );
        union.mount("tmp", "/tmp", true, Box::new(TmpFileSystem::new()));
        union.mount("private", "/.private", true, Box::new(fs_private));

        union.set_ctx(&caller_ctx);

        (AsyncifyFileSystem::new(union.clone()), union)
    };

    // Perform all the redirects
    for redirect in redirect.iter() {
        // If its not an absolutely path then make it one
        let mut filename = redirect.filename.clone();
        if filename.starts_with("/") == false {
            filename = format!("{}{}", ctx.working_dir, filename);
        }

        // Attempt to open the file
        let file = fs
            .new_open_options()
            .await
            .create(redirect.op.write())
            .create_new(redirect.op.write() && redirect.op.append() == false)
            .truncate(redirect.op.write() && redirect.op.append() == false)
            .append(redirect.op.append())
            .read(redirect.op.read())
            .write(redirect.op.write())
            .open(filename)
            .await;
        match file {
            Ok(mut file) => {
                // Open a new file description
                let (tx, mut rx, flag) = {
                    let (fd, tx, rx) = bidirectional_with_defaults(FdFlag::None);

                    // We now connect the newly opened file descriptor with the read file
                    let mut flag = FdFlag::None;
                    match redirect.fd {
                        -1 => {
                            if redirect.op.read() {
                                let mut fd = fd.clone();
                                flag = fd.set_flag(FdFlag::Stdin(false));
                                stdio.stdin = fd;
                            }
                            if redirect.op.write() {
                                let mut fd = fd.clone();
                                flag = fd.set_flag(FdFlag::Stdout(false));
                                stdio.stdout = fd;
                            }
                        }
                        0 => {
                            let mut fd = fd.clone();
                            flag = fd.set_flag(FdFlag::Stdin(false));
                            stdio.stdin = fd
                        }
                        1 => {
                            let mut fd = fd.clone();
                            flag = fd.set_flag(FdFlag::Stdout(false));
                            stdio.stdout = fd
                        }
                        2 => {
                            let mut fd = fd.clone();
                            flag = fd.set_flag(FdFlag::Stderr(false));
                            stdio.stderr = fd
                        }
                        _ => {
                            return on_early_exit(Some(format!("redirecting non-standard file descriptors is not yet supported")), err::ERR_EINVAL).await;
                        }
                    };

                    // Now we need to hook up the receiver and sender
                    (tx, rx, flag)
                };

                // Now hook up the sender and receiver on a shared task
                let system = ctx.system;
                let is_read = redirect.op.read();
                let is_write = redirect.op.write();
                system.fork_shared(move || async move {
                    if is_read {
                        while let Ok(read) = file.read(4096).await {
                            let _ = tx.send(FdMsg::new(read, flag)).await;
                        }
                    }
                    if is_write {
                        while let Some(msg) = rx.recv().await {
                            match msg {
                                FdMsg::Data { data, .. } => {
                                    let _ = file.write_all(data).await;
                                }
                                FdMsg::Flush { tx } => {
                                    file.flush().await;
                                    let _ = tx.send(()).await;
                                }
                            }
                        }
                        file.flush().await;
                    }
                });
            }
            Err(err) => {
                return on_early_exit(
                    Some(format!(
                        "failed to open the redirected file '{}' ({}): ",
                        redirect.filename, err
                    )),
                    match err {
                        FsError::EntityNotFound => ERR_ENOENT,
                        _ => ERR_EIO,
                    }
                )
                .await;
            }
        }
    }

    // Extract the bits we need from the eval context
    // as we will move it soon
    let sys = ctx.system;
    let mut stderr = ctx.stdio.stderr.clone();
    let bins = ctx.bins.clone();
    let compiler = ctx.compiler;
    let reactor = ctx.reactor.clone();
    let chroot = if chroot {
        Some(ctx.working_dir.clone())
    } else {
        None
    };

    // Create the process factory that used by this process to create sub-processes
    let sub_process_factory = ProcessExecFactory::new(
        ctx.abi.clone(),
        ctx.reactor.clone(),
        compiler,
        ctx.exec_factory.clone(),
        stdio.stdin.downgrade(),
        stdio.stdout.downgrade(),
        stdio.stderr.downgrade(),
        stdio.log.downgrade(),
        ctx
    );

    let forced_exit = caller_ctx.get_forced_exit();

    // The BUS pool is what gives this WASM process its syscall and operation system
    // functions and services
    let bus_thread_pool = WasmBusThreadPool::new(sub_process_factory, caller_ctx.clone());
    let bus_thread_pool_ret = Arc::clone(&bus_thread_pool);

    // This wait point is so that the main thread is created before it returns
    let (checkpoint_tx, mut checkpoint_rx) = mpsc::channel(1);

    // Spawn the process on a background thread
    let cmd = cmd.clone();
    let args = args.clone();
    let process_result = {
        let forced_exit = Arc::clone(&forced_exit);
        sys.spawn_stateful(move |mut thread_local| async move {
            let mut thread_local = thread_local.borrow_mut();

            // TODO: This caching of the modules was disabled as there is a bug within
            //       wasmer Module and JsValue that causes a panic in certain race conditions
            // Load or compile the module (they are cached in therad local storage)
            //let mut module = thread_local.modules.get_mut(&data_hash);
            #[cfg(not(feature = "cached_compiling"))]
            let mut module = None;

            if module.is_none() {
                if stderr.is_tty() {
                    let _ = stderr.write("Compiling...".as_bytes()).await;
                }

                // Choose the right compiler
                let store = match compiler {
                    #[cfg(feature = "cranelift")]
                    Compiler::Cranelift => {
                        let compiler = Cranelift::default();
                        Store::new(&Universal::new(compiler).engine())
                    }
                    #[cfg(feature = "llvm")]
                    Compiler::LLVM => {
                        let compiler = LLVM::default();
                        Store::new(&Universal::new(compiler).engine())
                    }
                    #[cfg(feature = "singlepass")]
                    Compiler::Singlepass => {
                        let compiler = Singlepass::default();
                        Store::new(&Universal::new(compiler).engine())
                    }
                    _ => Store::default(),
                };

                // Cache miss - compile the module
                info!("compiling {}", cmd);

                let store = Store::default();
                let compiled_module = match Module::new(&store, &data[..]) {
                    Ok(a) => a,
                    Err(err) => {
                        if stderr.is_tty() {
                            stderr.write_clear_line().await;
                        }
                        let _ = stderr
                            .write(format!("compile-error: {}\n", err).as_bytes())
                            .await;
                        let ctx = bus_thread_pool.take_context().unwrap();
                        return (ctx, ERR_ENOEXEC);
                    }
                };
                if stderr.is_tty() {
                    stderr.write_clear_line().await;
                }
                info!(
                    "compiled {}",
                    compiled_module.name().unwrap_or_else(|| cmd.as_str())
                );

                #[cfg(feature = "cached_compiling")]
                bins.set_compiled_module(data_hash.clone(), compiled_module.clone())
                    .await;

                thread_local
                    .modules
                    .insert(data_hash.clone(), compiled_module);

                module = thread_local.modules.get(&data_hash).map(|m| m.clone());
            }
            drop(thread_local);
            let mut module = module.unwrap();

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
                    let res = if let Some((alias, po_dir)) = pre_open.split_once(":") {
                        wasi_env.map_dir(alias, po_dir).is_ok()
                    } else {
                        wasi_env.preopen_dir(Path::new(pre_open.as_str())).is_ok()
                    };
                    if res == false {
                        if stderr.is_tty() {
                            stderr.write_clear_line().await;
                        }
                        let _ = stderr
                            .write(format!("pre-open error (path={})\n", pre_open).as_bytes())
                            .await;
                        let ctx = bus_thread_pool.take_context().unwrap();
                        return (ctx, ERR_ENOEXEC);
                    }
                }

            // Or we default and open the current directory
            } else if let Some(chroot) = chroot {
                wasi_env
                    .preopen_dir(Path::new(chroot.as_str()))
                    .unwrap()
                    .map_dir(".", Path::new(chroot.as_str()));
            } else {
                wasi_env.preopen_dir(Path::new("/")).unwrap();
            }

            // Add the tick callback that will invoke the WASM bus background
            // operations on the current thread
            {
                let bus_pool = Arc::clone(&bus_thread_pool);
                wasi_env.on_yield(move |thread| {
                    let forced_exit = forced_exit.load(Ordering::Acquire);
                    if forced_exit != 0 {
                        return Err(WasiError::Exit(forced_exit));
                    }
                    Ok(())
                });
            }

            // Finish off the WasiEnv
            let mut wasi_env = match wasi_env.set_fs(Box::new(union)).finalize() {
                Ok(a) => a,
                Err(err) => {
                    drop(module);
                    if stderr.is_tty() {
                        stderr.write_clear_line().await;
                    }
                    let _ = stderr
                        .write(format!("exec error: {}\n", err.to_string()).as_bytes())
                        .await;
                    let ctx = bus_thread_pool.take_context().unwrap();
                    return (ctx, ERR_ENOEXEC);
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
            let mut wasm_thread = bus_thread_pool.get_or_create(&wasi_thread);
            let wasm_bus_import = wasm_thread.import_object(&module);
            let import = wasi_import.chain_front(wasm_bus_import);

            // Let's instantiate the module with the imports.
            let instance = Instance::new(&module, &import).unwrap();
            
            // Let's call the `_start` function, which is our `main` function in Rust.
            let start = instance
                .exports
                .get_native_function::<(), ()>("_start")
                .ok();

            // We are ready for the checkpoint
            checkpoint_tx.send(()).await;

            // If there is a start function
            debug!("called main() on {}", cmd);
            let mut ret = if let Some(start) = start {
                match start.call() {
                    Ok(a) => err::ERR_OK,
                    Err(e) => match e.downcast::<WasiError>() {
                        Ok(WasiError::Exit(code)) => code,
                        Ok(WasiError::UnknownWasiVersion) => {
                            let _ = stderr
                                .write(
                                    &format!("exec-failed: unknown wasi version\n").as_bytes()
                                        [..],
                                )
                                .await;
                            err::ERR_ENOEXEC
                        }
                        Err(err) => err::ERR_PANIC,
                    },
                }
            } else {
                let _ = stderr
                    .write(&format!("exec-failed: missing _start function\n").as_bytes()[..])
                    .await;
                err::ERR_ENOEXEC
            };

            // If there is a polling worker that got registered then its time to consume
            // it which will effectively bring up a reactor based WASM module
            let worker = unsafe {
                let mut inner = wasm_thread.inner.lock();
                inner.poll_thread.take()
            };
            if let Some(worker) = worker {
                ret = worker.await;
            }

            // Ok we are done
            info!("exited (name={}) with code {}", cmd, ret);
            let ctx = bus_thread_pool.take_context().unwrap();
            (ctx, ret)
        })
    };

    // Wait for the checkpoint (either it triggers or it fails because its never reached
    // but whatever happens this checkpoint will be released)
    checkpoint_rx.recv().await;

    // Generate a PID for this process
    let (pid, process) = {
        let mut guard = reactor.write().await;
        let pid = guard.generate_pid(bus_thread_pool_ret.clone(), caller_ctx)?;
        let process = match guard.get_process(pid) {
            Some(a) => a,
            None => {
                return Err(ERR_ESRCH);
            }
        };
        (pid, process)
    };
    debug!("process created (pid={})", pid);

    Ok(( process, process_result, bus_thread_pool_ret))
}
