#![allow(dead_code)]
#![allow(unused)]
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sha2::digest::generic_array::sequence::Lengthen;
use wasmer::Extern;
use wasmer::ExternType;
use wasmer::ImportType;
use wasmer::Memory;
use wasmer::MemoryType;
use wasmer::Pages;
use wasmer_wasi::import_object_for_all_wasi_versions;
use std::collections::HashMap;
use std::future::Future;
use std::io::Read;
use std::ops::Deref;
use std::ops::DerefMut;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::Context;
use std::task::Poll;
use std::time::Duration;
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
use crate::wasmer::{Imports, Instance, Module, Store};
use crate::wasmer_vfs::FileSystem;
use crate::wasmer_vfs::FsError;
use crate::wasmer_wasi::Stdin;
use crate::wasmer_wasi::{Stdout, WasiError, WasiState};

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

    let (process, process_result, _, _) =
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
) -> Result<
    (
        Process,
        AsyncResult<(EvalContext, u32)>,
        Arc<WasiRuntime>,
        Arc<WasmCheckpoint>,
    ),
    u32,
> {
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
    let mut envs = ctx.env.iter().filter_map(|(k, _)| ctx.env.get(k.as_str()).map(|v| (k.clone(), v))).collect::<HashMap<_,_>>();
    let mut set_pwd = false;
    let mut base_dir = None;
    let mut chroot = ctx.chroot;
    let (program_data_hash, program_data, mut fs_private) = match load_bin(&ctx, cmd, &mut stdio).await {
        Some(a) => {
            if a.chroot {
                chroot = true;
            }
            if let Some(d) = a.base_dir {
                base_dir = Some(d);
            }
            for (k, v) in a.envs {
                if k == "PWD" {
                    set_pwd = true;
                }
                envs.insert(k, v);
            }
            for mapping in a.mappings {
                preopen.push(mapping);
            }

            (a.hash, a.data, a.fs)
        }
        None => {
            return on_early_exit(None, err::ERR_ENOENT).await;
        }
    };
    let pwd = ctx.working_dir.clone();
    if set_pwd == false {
        envs.insert("PWD".to_string(), pwd.clone());
    };

    // Create a store for the module and memory
    #[cfg(feature = "sys")]
    let store = match ctx.engine.clone() {
        Some(engine) => Store::new(engine),
        None => Store::default()
    };
    #[cfg(feature = "js")]
    let store = Store::default();

    // Load or compile the module (they are cached in thread local storage)
    // If compile caching is enabled the load the module
    let cached_module = { ctx.bins.get_compiled_module(&store, &program_data_hash, ctx.compiler).await };

    // This wait point is so that the main thread is created before it returns
    let (checkpoint1_tx, mut checkpoint1) = ctx.checkpoint1.take().unwrap_or_else(|| WasmCheckpoint::new());
    let (checkpoint2_tx, mut checkpoint2) = ctx.checkpoint2.take().unwrap_or_else(|| WasmCheckpoint::new());

    // We listen for any forced exits using this channel
    let caller_ctx = WasmCallerContext::new(&checkpoint2);
    fs_private.set_ctx(&caller_ctx);

    // Create the filesystem
    let (fs, union_base) = {
        let stdio = stdio.clone();
        let mut union = ctx.root.clone();
        union.mount("proc", "/dev", true, Box::new(ProcFileSystem::new(stdio)), None);
        union.mount("tmp", "/tmp", true, Box::new(TmpFileSystem::new()), None);
        union.mount("private", "/.private", true, Box::new(fs_private), None);
        union.set_ctx(&caller_ctx);
        
        (AsyncifyFileSystem::new(union.clone()), union)
    };

    // Extra preopens that should in-fact be file system mappings
    let mut union = UnionFileSystem::new();
    if ctx.chroot == false && preopen.len() > 0 {
        union.mount("root", "/", true, Box::new(union_base.clone()), None);
        for pre_open in preopen.iter() {
            if let Some((alias, po_dir)) = pre_open.split_once(":") {
                if alias.starts_with("./") { continue; }
                let name = format!("mapping{}", fastrand::u64(..));
                union.mount(name.as_str(), alias, true, Box::new(union_base.clone()), Some(po_dir));
            }
        }
    } else {
        union = union_base;
    }

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
                    },
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
    let reactor = ctx.reactor.clone();
    let chroot = if chroot {
        Some(ctx.working_dir.clone())
    } else {
        None
    };

    // If no module is loaded then we need to compile it again
    let module = match cached_module {
        Some(a) => a,
        None => {
            if stderr.is_tty() {
                let _ = stderr.write("Compiling...".as_bytes()).await;
                let _ = stderr.flush_async().await;
            }

            // Cache miss - compile the module
            debug!("compiling {}", cmd);
            let compiled_module = match Module::new(&store, &program_data[..]) {
                Ok(a) => a,
                Err(err) => {
                    if stderr.is_tty() {
                        stderr.write_clear_line().await;
                    }
                    let _ = stderr
                        .write(format!("compile-error: {}\n", err).as_bytes())
                        .await;
                    return on_early_exit(None, err::ERR_ENOEXEC).await;
                }
            };
            if stderr.is_tty() {
                stderr.write_clear_line().await;
            }
            info!(
                "compiled {}",
                compiled_module.name().unwrap_or_else(|| cmd.as_str())
            );

            bins.set_compiled_module(program_data_hash.clone(), ctx.compiler, &compiled_module)
                .await;

            compiled_module
        }
    };

    // Determine if shared memory needs to be created and imported
    let shared_memory = module
        .imports()
        .memories()
        .next()
        .map(|a| *a.ty());

    // Determine if we are going to create memory and import it or just rely on self creation of memory
    let memory_spawn = match shared_memory {
        Some(ty) => {
            #[cfg(feature = "sys")]
            let style = match &ctx.engine {
                    Some(engine) => Store::new(engine.clone()),
                    None => Store::default()
                }
                .tunables()
                .memory_style(&ty);            
            SpawnType::CreateWithType(SpawnedMemory {
                ty,
                #[cfg(feature = "sys")]
                style
            })
        },
        None => SpawnType::Create,
    };

    // Create the process factory that used by this process to create sub-processes
    let launch_env = LaunchEnvironment {
        abi: ctx.abi.clone(),
        inherit_stdin: stdio.stdin.downgrade(),
        inherit_stdout: stdio.stdout.downgrade(),
        inherit_stderr: stdio.stderr.downgrade(),
        inherit_log: stdio.log.downgrade(),
    };
    let sub_process_factory = ProcessExecFactory::new(
        ctx.reactor.clone(),
        #[cfg(feature = "sys")]
        ctx.engine.clone(),
        ctx.compiler,
        ctx.exec_factory.clone(),
        ctx,
    );
    
    let forced_exit = caller_ctx.get_forced_exit();

    // Create the runtime that will perform terminal specific actions
    let wasi_runtime = Arc::new(WasiRuntime::new(
        &forced_exit,
        sub_process_factory,
        caller_ctx.clone()));
    let ctx_taker = wasi_runtime.prepare_take_context();

    // Spawn the process on a background thread
    let cmd = cmd.clone();
    let args = args.clone();
    let process_result = {
        let wasi_runtime = wasi_runtime.clone();
        let forced_exit = Arc::clone(&forced_exit);

        sys.spawn_wasm(move |mut store, module, memory| async move
        {
            // Build the list of arguments
            let args = args.iter().skip(1).map(|a| a.as_str()).collect::<Vec<_>>();
            let envs = envs.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect::<HashMap<_,_>>();
            
            // Create the `WasiEnv`.
            let mut wasi_env = WasiState::new(cmd.as_str());
            let mut wasi_env = wasi_env
                .args(&args)
                .envs(&envs)
                .stdin(Box::new(stdio.stdin.clone()))
                .stdout(Box::new(stdio.stdout.clone()))
                .stderr(Box::new(stdio.stderr.clone()))
                .set_fs(Box::new(union))
                .setup_fs(Box::new(move |_, fs| {
                    fs.set_current_dir(pwd.as_str());
                    Ok(())
                }));

            // We default and open the current directory
            let mut is_chroot = false;
            if let Some(chroot) = chroot {
                is_chroot = true;
                wasi_env
                    .preopen_dir(Path::new(chroot.as_str()))
                    .unwrap()
                    .map_dir(".", Path::new(chroot.as_str()));
            } else {
                wasi_env
                    .preopen_dir(Path::new("/"))
                    .unwrap()
                    .map_dir(".", "/");
            }

            // Add the extra pre-opens
            if preopen.len() > 0 {
                for pre_open in preopen {
                    let res = if let Some((alias, po_dir)) = pre_open.split_once(":") {
                        if alias.starts_with("./") || is_chroot == true {
                            wasi_env.map_dir(alias, po_dir).is_ok()
                        } else {
                            // in certain scenarios the map_dir doesnt appear to work properly thus we
                            // avoid it and apply the mappings at the virtual file system
                            continue;
                        }
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
                        let ctx = ctx_taker.take_context().unwrap();
                        return (ctx, ERR_ENOEXEC);
                    }
                }
            }

            // Create a new runtime
            // (we do a deeper clone as its needed to implement the trait
            //  however everyone else will just use arc reference counting
            //  for better performance)
            {
                let wasi_runtime = wasi_runtime.deref();
                wasi_env.runtime(wasi_runtime.clone());
            }            

            // Build the environment
            let mut wasi_env = match wasi_env.finalize(&mut store) {
                Ok(a) => a,
                Err(err) => {
                    drop(module);
                    if stderr.is_tty() {
                        stderr.write_clear_line().await;
                    }
                    let _ = stderr
                        .write(format!("exec error: {}\n", err.to_string()).as_bytes())
                        .await;
                    let ctx = ctx_taker.take_context().unwrap();
                    return (ctx, ERR_ENOEXEC);
                }
            };
            
            // Let's instantiate the module with the imports.
            let mut import_object = import_object_for_all_wasi_versions(&mut store, &wasi_env.env);
            if let Some(memory) = memory {
                import_object.define("env", "memory", Memory::new_from_existing(&mut store, memory));
            }
            let instance = match Instance::new(&mut store, &module, &import_object) {
                Ok(a) => a,
                Err(err) => {
                    let _ = stderr.write(format!("instantiate error ({})\n", err.to_string()).as_bytes()).await;
                    let ctx = ctx_taker.take_context().unwrap();
                    return (ctx, ERR_ENOEXEC);
                }
            };

            // Initialize the WASI environment
            if let Err(err) = wasi_env.initialize(&mut store, &instance) {
                let _ = stderr.write(format!("instantiate error ({})\n", err.to_string()).as_bytes()).await;
                let ctx = ctx_taker.take_context().unwrap();
                return (ctx, ERR_ENOEXEC);
            }

            // If this module exports an _initialize function, run that first.
            if let Ok(initialize) = instance.exports.get_function("_initialize") {
                if let Err(e) = initialize.call(&mut store, &[]) {
                    let ctx = ctx_taker.take_context().unwrap();
                    return (ctx, match e.downcast::<WasiError>() {
                        Ok(WasiError::Exit(code)) => code,
                        Ok(WasiError::UnknownWasiVersion) => {
                            let _ = stderr
                                .write(
                                    &format!("exec-failed: unknown wasi version\n").as_bytes()[..],
                                )
                                .await;
                            err::ERR_ENOEXEC
                        }
                        Err(err) => err::ERR_PANIC,
                    });
                }
            }

            // Let's call the `_start` function, which is our `main` function in Rust.
            let start = instance
                .exports
                .get_function("_start")
                .ok();

            // The first checkpoint is just before we invoke the start method
            checkpoint1_tx.send(()).await;

            // If there is a start function
            debug!("called main() on {}", cmd);
            let mut ret = if let Some(start) = start {
                match start.call(&mut store, &[]) {
                    Ok(a) => err::ERR_OK,
                    Err(e) => match e.downcast::<WasiError>() {
                        Ok(WasiError::Exit(code)) => code,
                        Ok(WasiError::UnknownWasiVersion) => {
                            let _ = stderr
                                .write(
                                    &format!("exec-failed: unknown wasi version\n").as_bytes()[..],
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
            debug!("main() has exited on {}", cmd);

            // The second checkpoint is after the start method completes but before
            // all the background threads have exited
            checkpoint2_tx.send(()).await;

            // Force everything to exit (if it has not already)
            forced_exit.compare_exchange(0, err::ERR_EINTR, Ordering::AcqRel, Ordering::Relaxed);

            // If the main thread exited normally and there are still active threads
            // running then we need to wait for them all to exit before we terminate
            if ret == 0 {
                while wasi_env.data(&store).active_threads() > 0 {
                    if let Err(err) = wasi_env.data(&store).sleep(Duration::from_millis(50)) {
                        match err {
                            WasiError::Exit(code) => {
                                if code != err::ERR_EINTR {
                                    ret = code;
                                }
                            },
                            _ => { }
                        }
                        break;
                    }
                }
            }

            // Ok we are done
            debug!("exited (name={}) with code {}", cmd, ret);
            let ctx = ctx_taker.take_context().unwrap();
            (ctx, ret)
        }, store, module, memory_spawn)
    };

    // Wait for the checkpoint (either it triggers or it fails because its never reached
    // but whatever happens this checkpoint will be released)
    checkpoint1.wait().await;

    // Generate a PID for this process
    let (pid, process) = {
        let mut guard = reactor.write().await;
        let pid = guard.generate_pid(caller_ctx)?;
        let process = match guard.get_process(pid) {
            Some(a) => a,
            None => {
                return Err(ERR_ESRCH);
            }
        };
        (pid, process)
    };
    debug!("process created (pid={})", pid);

    Ok((process, process_result, wasi_runtime, checkpoint2))
}
