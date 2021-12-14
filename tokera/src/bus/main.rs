use crate::opt::OptsBus;
use ate::prelude::*;
use ate_auth::prelude::*;
use ate_files::codes::*;
use ate_files::prelude::*;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::mpsc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::backend::fuse as backend;
use wasm_bus::backend::fuse::*;
use wasm_bus::fuse::FsError;
use wasm_bus::fuse::FsResult;
use wasm_bus::prelude::*;

pub async fn main_opts_bus(
    opts: OptsBus,
    conf: AteConfig,
    token_path: String,
    auth_url: url::Url,
) -> Result<(), crate::error::BusError> {
    info!("wasm bus initializing");

    // Load the session
    let session_user = match main_session_user(None, Some(token_path.clone()), None).await {
        Ok(a) => a,
        Err(err) => {
            warn!("failed to acquire token - {}", err);
            return Err(crate::error::BusErrorKind::LoginFailed.into());
        }
    };

    // Build the configuration used to access the chains
    let mut conf = conf.clone();
    conf.configured_for(opts.configured_for);
    conf.log_format.meta = opts.meta_format;
    conf.log_format.data = opts.data_format;
    conf.recovery_mode = opts.recovery_mode;
    conf.compact_mode = opts
        .compact_mode
        .with_growth_factor(opts.compact_threshold_factor)
        .with_growth_size(opts.compact_threshold_size)
        .with_timer_value(Duration::from_secs(opts.compact_timer));

    // Create the registry
    let registry = Arc::new(Registry::new(&conf).await);

    // Register all the functions
    listen(move |handle: CallHandle, mount: Mount| {
        // Derive the group from the mount address
        let mut group = None;
        if let Some((group_str, _)) = mount.name.split_once("/") {
            group = Some(group_str.to_string());
        }

        let session_user = session_user.clone();
        let remote = opts.remote.clone();
        let registry = registry.clone();
        let auth_url = auth_url.clone();
        async move {
            // Attempt to grab additional permissions for the group (if it has any)
            let session: AteSessionType = if group.is_some() {
                match main_gather(
                    group.clone(),
                    session_user.clone().into(),
                    auth_url,
                    "Group",
                )
                .await
                {
                    Ok(a) => a.into(),
                    Err(err) => {
                        debug!("Group authentication failed: {} - falling back to user level authorization", err);
                        session_user.into()
                    }
                }
            } else {
                session_user.into()
            };

            // Build the request context
            let mut context = RequestContext::default();
            context.uid = session.uid().unwrap_or_default();
            context.gid = session.gid().unwrap_or_default();

            // Load the chain
            let key = ChainKey::from(mount.name.clone());
            let chain = match registry.open(&remote, &key).await {
                Ok(a) => a,
                Err(err) => {
                    warn!("failed to open chain - {}", err);
                    return;
                }
            };
            let accessor = Arc::new(
                FileAccessor::new(
                    chain.as_arc(),
                    group,
                    session,
                    TransactionScope::Local,
                    TransactionScope::Local,
                    false,
                    false,
                )
                .await,
            );

            // Add all the operations
            {
                let accessor = accessor.clone();
                respond_to(handle, move |_handle, meta: ReadMetadata| {
                    debug!("bus::read-metadata(path={})", meta.path);
                    let accessor = accessor.clone();
                    async move {
                        if let Ok(Some(file)) = accessor.search(&context, meta.path.as_str()).await
                        {
                            FsResult::Ok(conv_meta(file))
                        } else {
                            FsResult::Err(FsError::EntityNotFound)
                        }
                    }
                });
            }
            {
                let accessor = accessor.clone();
                respond_to(handle, move |_handle, meta: ReadSymlinkMetadata| {
                    debug!("bus::read-symlink-metadata(path={})", meta.path);
                    let accessor = accessor.clone();
                    async move {
                        if let Ok(Some(file)) = accessor.search(&context, meta.path.as_str()).await
                        {
                            FsResult::Ok(conv_meta(file))
                        } else {
                            FsResult::Err(FsError::EntityNotFound)
                        }
                    }
                });
            }
            {
                let accessor = accessor.clone();
                respond_to(handle, move |_handle, read_dir: ReadDir| {
                    debug!("bus::read-dir(path={})", read_dir.path);
                    let accessor = accessor.clone();
                    async move {
                        if let Ok(Some(file)) =
                            accessor.search(&context, read_dir.path.as_str()).await
                        {
                            if let Ok(fh) =
                                accessor.opendir(&context, file.ino, O_RDONLY as u32).await
                            {
                                let mut ret = backend::Dir::default();
                                for entry in fh.children.iter() {
                                    if entry.name == "." || entry.name == ".." {
                                        continue;
                                    }
                                    trace!("bus::read-dir::found - {}", entry.name);
                                    ret.data.push(backend::DirEntry {
                                        path: entry.name.clone(),
                                        metadata: Some(conv_meta(entry.attr.clone())),
                                    });
                                }
                                let _ = accessor.releasedir(&context, file.ino, fh.fh, 0).await;
                                FsResult::Ok(ret)
                            } else {
                                FsResult::Err(FsError::IOError)
                            }
                        } else {
                            FsResult::Err(FsError::EntityNotFound)
                        }
                    }
                });
            }
            {
                let accessor = accessor.clone();
                respond_to(handle, move |_handle, create_dir: CreateDir| {
                    debug!("bus::create-dir(path={})", create_dir.path);
                    let accessor = accessor.clone();
                    async move {
                        let path = std::path::Path::new(&create_dir.path);
                        let name = path.file_name().ok_or_else(|| FsError::InvalidInput)?;
                        let parent = path.parent().ok_or_else(|| FsError::InvalidInput)?;
                        if let Ok(Some(parent)) = accessor
                            .search(&context, parent.to_string_lossy().as_ref())
                            .await
                        {
                            let attr = accessor
                                .mkdir(
                                    &context,
                                    parent.ino,
                                    name.to_string_lossy().as_ref(),
                                    parent.mode,
                                )
                                .await
                                .map_err(|_| FsError::IOError)?;
                            Ok(conv_meta(attr))
                        } else {
                            Err(FsError::EntityNotFound)
                        }
                    }
                });
            }
            {
                let accessor = accessor.clone();
                respond_to(handle, move |_handle, remove_dir: RemoveDir| {
                    debug!("bus::remove-dir(path={})", remove_dir.path);
                    let accessor = accessor.clone();
                    async move {
                        let path = std::path::Path::new(&remove_dir.path);
                        let name = path.file_name().ok_or_else(|| FsError::InvalidInput)?;
                        let parent = path.parent().ok_or_else(|| FsError::InvalidInput)?;
                        if let Ok(Some(parent)) = accessor
                            .search(&context, parent.to_string_lossy().as_ref())
                            .await
                        {
                            let _ = accessor
                                .rmdir(&context, parent.ino, name.to_string_lossy().as_ref())
                                .await;
                            Ok(())
                        } else {
                            Err(FsError::EntityNotFound)
                        }
                    }
                });
            }
            {
                let accessor = accessor.clone();
                respond_to(handle, move |_handle, rename: Rename| {
                    debug!("bus::rename(from={}, to={})", rename.from, rename.to);
                    let accessor = accessor.clone();
                    async move {
                        let path = std::path::Path::new(&rename.from);
                        let new_path = std::path::Path::new(&rename.to);
                        let name = path.file_name().ok_or_else(|| FsError::InvalidInput)?;
                        let new_name = new_path.file_name().ok_or_else(|| FsError::InvalidInput)?;
                        let parent = path.parent().ok_or_else(|| FsError::InvalidInput)?;
                        let new_parent = new_path.parent().ok_or_else(|| FsError::InvalidInput)?;
                        if let Ok(Some(parent)) = accessor
                            .search(&context, parent.to_string_lossy().as_ref())
                            .await
                        {
                            if let Ok(Some(new_parent)) = accessor
                                .search(&context, new_parent.to_string_lossy().as_ref())
                                .await
                            {
                                let _ = accessor
                                    .rename(
                                        &context,
                                        parent.ino,
                                        name.to_string_lossy().as_ref(),
                                        new_parent.ino,
                                        new_name.to_string_lossy().as_ref(),
                                    )
                                    .await;
                                Ok(())
                            } else {
                                Err(FsError::EntityNotFound)
                            }
                        } else {
                            Err(FsError::EntityNotFound)
                        }
                    }
                });
            }
            {
                let accessor = accessor.clone();
                respond_to(handle, move |_handle, remove_file: RemoveFile| {
                    debug!("bus::remove_file(path={})", remove_file.path);
                    let accessor = accessor.clone();
                    async move {
                        let path = std::path::Path::new(&remove_file.path);
                        let name = path.file_name().ok_or_else(|| FsError::InvalidInput)?;
                        let parent = path.parent().ok_or_else(|| FsError::InvalidInput)?;
                        if let Ok(Some(parent)) = accessor
                            .search(&context, parent.to_string_lossy().as_ref())
                            .await
                        {
                            let _ = accessor
                                .unlink(&context, parent.ino, name.to_string_lossy().as_ref())
                                .await;
                            Ok(())
                        } else {
                            Err(FsError::EntityNotFound)
                        }
                    }
                });
            }
            {
                let accessor = accessor.clone();
                respond_to(handle, move |handle, new_open: NewOpen| {
                    let accessor = accessor.clone();
                    async move {
                        // Determine all the flags
                        let mut flags = 0i32;
                        if new_open.append { flags |= O_APPEND; }
                        if new_open.create { flags |= O_CREAT; }
                        if new_open.create_new { flags |= O_CREAT | O_TRUNC; }
                        if new_open.read && new_open.write { flags |= O_RDWR; }
                        else if new_open.read { flags |= O_RDONLY; }
                        else if new_open.write { flags |= O_WRONLY; }
                        if new_open.truncate { flags |= O_TRUNC; }
                        let append = new_open.append;
                        let create = new_open.create | new_open.create_new;

                        // Every file as an offset pointer
                        let offset = Arc::new(Mutex::new(0u64));
                        
                        // We either receive the open or close command
                        let (tx_close, mut rx_close) = mpsc::channel::<()>(1);
                        {
                            let tx_close = tx_close.clone();
                            let accessor = accessor.clone();
                            respond_to(handle, move |_, open: Open| {
                                debug!("bus::open(path={})", open.path);
                                let offset = offset.clone();
                                let accessor = accessor.clone();  
                                let tx_close = tx_close.clone();
                                async move {
                                    let file = if let Ok(Some(file)) = accessor.search(&context, open.path.as_str()).await {
                                        match accessor.open(&context, file.ino, flags as u32).await {
                                            Ok(a) => a,
                                            Err(err) => {
                                                debug!("open failed (path={}) - {}", open.path, err);
                                                return FsResult::Err(FsError::IOError);
                                            }
                                        }
                                    } else if create == true {
                                        let path = std::path::Path::new(&open.path);
                                        let name = path.file_name().ok_or_else(|| FsError::InvalidInput)?;
                                        let parent = path.parent().ok_or_else(|| FsError::InvalidInput)?;
                                        if let Ok(Some(parent)) = accessor
                                            .search(&context, parent.to_string_lossy().as_ref())
                                            .await
                                        {
                                            match accessor.create(&context, parent.ino, name.to_string_lossy().as_ref(), 0o666 as u32).await {
                                                Ok(a) => a,
                                                Err(err) => {
                                                    debug!("open failed (path={}) - {}", open.path, err);
                                                    return FsResult::Err(FsError::IOError);
                                                }
                                            }
                                        } else {
                                            return FsResult::Err(FsError::EntityNotFound);
                                        }
                                    } else {
                                        return FsResult::Err(FsError::EntityNotFound);
                                    };

                                    if append {
                                        *(offset.lock().unwrap()) = file.spec.size();
                                    }

                                    {
                                        let path = open.path.clone();
                                        let accessor = accessor.clone();
                                        respond_to(handle, move |_handle, _unlink: Unlink| {
                                            debug!("bus::unlink (path={})", path);
                                            let path = path.clone();
                                            let accessor = accessor.clone();
                                            async move {
                                                let path = std::path::Path::new(&path);
                                                let name = path.file_name().ok_or_else(|| FsError::InvalidInput)?;
                                                let parent = path.parent().ok_or_else(|| FsError::InvalidInput)?;
                                                if let Ok(Some(parent)) = accessor
                                                    .search(&context, parent.to_string_lossy().as_ref())
                                                    .await
                                                {
                                                    let _ = accessor
                                                        .unlink(&context, parent.ino, name.to_string_lossy().as_ref())
                                                        .await;
                                                    Ok(())
                                                } else {
                                                    Err(FsError::EntityNotFound)
                                                }
                                            }
                                        });
                                    }

                                    {
                                        let file = file.clone();
                                        respond_to(
                                            handle,
                                            move |_handle, set_length: SetLength| {
                                                debug!("bus::set-length(len={})", set_length.len);
                                                let file = file.clone();
                                                async move {
                                                    if let Ok(_) = file.spec.fallocate(set_length.len).await {
                                                        Ok(())
                                                    } else {
                                                        Err(FsError::IOError)
                                                    }
                                                }
                                            },
                                        );
                                    }

                                    {
                                        let offset = offset.clone();
                                        let file = file.clone();
                                        respond_to(handle, move |_handle, seek: Seek| {
                                            debug!("bus::seek({:?})", seek);
                                            let offset = offset.clone();
                                            let file = file.clone();
                                            async move {
                                                let mut offset = offset.lock().unwrap();
                                                match seek {
                                                    Seek::Current(a) if a > 0 => {
                                                        if let Some(a) = offset.checked_add(a.abs() as u64) {
                                                            *offset = a;
                                                        } else {
                                                            return Err(FsError::InvalidInput);
                                                        }
                                                    }
                                                    Seek::Current(a) if a < 0 => {
                                                        if let Some(a) = offset.checked_sub(a.abs() as u64) {
                                                            *offset = a;
                                                        } else {
                                                            return Err(FsError::InvalidInput);
                                                        }
                                                    }
                                                    Seek::Current(_) => { }
                                                    Seek::End(a) if a > 0 => {
                                                        if let Some(a) = file.spec.size().checked_add(a.abs() as u64) {
                                                            *offset = a;
                                                        } else {
                                                            return Err(FsError::InvalidInput);
                                                        }
                                                    }
                                                    Seek::End(a) if a < 0 => {
                                                        if let Some(a) = file.spec.size().checked_sub(a.abs() as u64) {
                                                            *offset = a;
                                                        } else {
                                                            return Err(FsError::InvalidInput);
                                                        }
                                                    }
                                                    Seek::End(_) => { }
                                                    Seek::Start(a) => {
                                                        *offset = a;                                                   
                                                    }
                                                }
                                                Ok(*offset)
                                            }
                                        });
                                    }

                                    {
                                        let offset = offset.clone();
                                        let file = file.clone();
                                        respond_to(handle, move |_handle, write: Write| {
                                            debug!("bus::write({} bytes)", write.data.len());
                                            let offset = offset.clone();
                                            let file = file.clone();
                                            async move {
                                                let offset = {
                                                    let mut offset = offset.lock().unwrap();
                                                    if append {
                                                        *offset = file.spec.size();
                                                    }
                                                    *offset
                                                };

                                                error!("ARRRRR!");
                                                file.spec.write(offset, &write.data[..]).await
                                                    .map_err(|err| {
                                                        debug!("write failed - {}", err);
                                                        FsError::IOError
                                                    })
                                            }
                                        });
                                    }

                                    {
                                        let offset = offset.clone();
                                        let file = file.clone();
                                        respond_to(handle, move |_handle, read: Read| {
                                            debug!("bus::read({} bytes)", read.len);
                                            let offset = offset.clone();
                                            let file = file.clone();
                                            async move {
                                                let offset = offset.lock().unwrap().clone();
                                                file.spec.read(offset, read.len).await
                                                    .map_err(|err| {
                                                        debug!("read failed - {}", err);
                                                        FsError::IOError
                                                    })
                                                    .map(|a| a.to_vec())
                                            }
                                        });
                                    }

                                    {
                                        let file = file.clone();
                                        respond_to(handle, move |_handle, _flush: Flush| {
                                            debug!("bus::flush");
                                            let file = file.clone();
                                            async move {
                                                file.spec.commit().await
                                                    .map_err(|err| {
                                                        debug!("flush failed - {}", err);
                                                        FsError::IOError
                                                    })
                                            }
                                        });
                                    }

                                    // The file will shutdown when an close command is received
                                    {
                                        let file = file.clone();
                                        respond_to(handle, move |_handle, _close: Close| {
                                            debug!("bus::close");
                                            let file = file.clone();
                                            let tx_close = tx_close.clone();
                                            async move {
                                                let _ = file.spec.commit().await;
                                                let _ = tx_close.send(()).await;
                                            }
                                        });
                                    }
                                    FsResult::Ok(conv_meta(file.attr.clone()))
                                }
                            });
                        }

                        let _ = rx_close.recv().await;

                        // We need to make sure the data is all sent
                        let _ = accessor.sync_all().await;
                    }
                });
            }

            // The mount will shutdown when an Unmount command is received
            let (tx_unmount, mut rx_unmount) = mpsc::channel::<()>(1);
            respond_to(handle, move |_handle, _meta: Unmount| {
                let tx = tx_unmount.clone();
                async move {
                    let _ = tx.send(()).await;
                }
            });

            // We are now running
            info!("successfully mounted {}", mount.name);
            let _ = rx_unmount.recv().await;
        }
    });

    // Enter a polling loop
    serve();
    Ok(())
}

fn conv_file_type(kind: ate_files::api::FileKind) -> backend::FileType {
    let mut ret = backend::FileType::default();
    match kind {
        ate_files::api::FileKind::Directory => {
            ret.dir = true;
        }
        ate_files::api::FileKind::RegularFile => {
            ret.file = true;
        }
        ate_files::api::FileKind::FixedFile => {
            ret.file = true;
        }
        ate_files::api::FileKind::SymLink => {
            ret.symlink = true;
        }
    }
    ret
}

fn conv_meta(file: ate_files::attr::FileAttr) -> backend::Metadata {
    backend::Metadata {
        ft: conv_file_type(file.kind),
        accessed: file.accessed,
        created: file.created,
        modified: file.updated,
        len: file.size,
    }
}
