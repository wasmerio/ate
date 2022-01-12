use async_trait::async_trait;
use ate_files::codes::*;
use ate_files::prelude::*;
use derivative::*;
use std::sync::Arc;
use std::sync::Mutex;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus_fuse::api;
use wasm_bus_fuse::prelude::*;

use super::conv_err;
use super::opened_file::*;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct FileSystem {
    #[derivative(Debug = "ignore")]
    accessor: Arc<FileAccessor>,
    context: RequestContext,
}

impl FileSystem {
    pub fn new(accessor: Arc<FileAccessor>, context: RequestContext) -> FileSystem {
        FileSystem { accessor, context }
    }
}

#[async_trait]
impl api::FileSystemSimplified for FileSystem {
    async fn init(&self) -> FsResult<()> {
        self.accessor.init(&self.context).await.map_err(conv_err)?;
        Ok(())
    }

    async fn read_dir(&self, path: String) -> FsResult<api::Dir> {
        if let Ok(Some(file)) = self.accessor.search(&self.context, path.as_str()).await {
            match self
                .accessor
                .opendir(&self.context, file.ino, O_RDONLY as u32)
                .await
            {
                Ok(fh) => {
                    let mut ret = api::Dir::default();
                    for entry in fh.children.iter() {
                        if entry.name == "." || entry.name == ".." {
                            continue;
                        }
                        trace!("bus::read-dir::found - {}", entry.name);
                        ret.data.push(api::DirEntry {
                            path: entry.name.clone(),
                            metadata: Some(super::conv_meta(entry.attr.clone())),
                        });
                    }
                    let _ = self
                        .accessor
                        .releasedir(&self.context, file.ino, fh.fh, 0)
                        .await;
                    FsResult::Ok(ret)
                }
                Err(err) => {
                    debug!("read_dir failed - {}", err);
                    FsResult::Err(conv_err(err))
                }
            }
        } else {
            debug!("read_dir failed - not found ({})", path);
            FsResult::Err(FsError::EntityNotFound)
        }
    }

    async fn create_dir(&self, path: String) -> FsResult<api::Metadata> {
        let path = std::path::Path::new(&path);
        let name = path.file_name().ok_or_else(|| FsError::InvalidInput)?;
        let parent = path.parent().ok_or_else(|| FsError::InvalidInput)?;
        if let Ok(Some(parent)) = self
            .accessor
            .search(&self.context, parent.to_string_lossy().as_ref())
            .await
        {
            self.accessor
                .mkdir(
                    &self.context,
                    parent.ino,
                    name.to_string_lossy().as_ref(),
                    parent.mode,
                )
                .await
                .map_err(|err| {
                    debug!("create_dir failed - {}", err);
                    conv_err(err)
                })
                .map(super::conv_meta)
        } else {
            debug!(
                "create_dir failed - parent not found ({})",
                parent.to_string_lossy()
            );
            Err(FsError::EntityNotFound)
        }
    }

    async fn remove_dir(&self, path: String) -> FsResult<()> {
        let path = std::path::Path::new(&path);
        let name = path.file_name().ok_or_else(|| FsError::InvalidInput)?;
        let parent = path.parent().ok_or_else(|| FsError::InvalidInput)?;
        if let Ok(Some(parent)) = self
            .accessor
            .search(&self.context, parent.to_string_lossy().as_ref())
            .await
        {
            let _ = self
                .accessor
                .rmdir(&self.context, parent.ino, name.to_string_lossy().as_ref())
                .await;
            Ok(())
        } else {
            debug!(
                "remove_dir failed - parent not found ({})",
                parent.to_string_lossy()
            );
            Err(FsError::EntityNotFound)
        }
    }

    async fn rename(&self, from: String, to: String) -> FsResult<()> {
        let path = std::path::Path::new(&from);
        let new_path = std::path::Path::new(&to);
        let name = path.file_name().ok_or_else(|| FsError::InvalidInput)?;
        let new_name = new_path.file_name().ok_or_else(|| FsError::InvalidInput)?;
        let parent = path.parent().ok_or_else(|| FsError::InvalidInput)?;
        let new_parent = new_path.parent().ok_or_else(|| FsError::InvalidInput)?;
        if let Ok(Some(parent)) = self
            .accessor
            .search(&self.context, parent.to_string_lossy().as_ref())
            .await
        {
            if let Ok(Some(new_parent)) = self
                .accessor
                .search(&self.context, new_parent.to_string_lossy().as_ref())
                .await
            {
                let _ = self
                    .accessor
                    .rename(
                        &self.context,
                        parent.ino,
                        name.to_string_lossy().as_ref(),
                        new_parent.ino,
                        new_name.to_string_lossy().as_ref(),
                    )
                    .await;
                Ok(())
            } else {
                debug!("remove_dir failed - new parent not found");
                Err(FsError::EntityNotFound)
            }
        } else {
            debug!("rename failed - parent not found");
            Err(FsError::EntityNotFound)
        }
    }

    async fn remove_file(&self, path: String) -> FsResult<()> {
        let path = std::path::Path::new(&path);
        let name = path.file_name().ok_or_else(|| FsError::InvalidInput)?;
        let parent = path.parent().ok_or_else(|| FsError::InvalidInput)?;
        if let Ok(Some(parent)) = self
            .accessor
            .search(&self.context, parent.to_string_lossy().as_ref())
            .await
        {
            let _ = self
                .accessor
                .unlink(&self.context, parent.ino, name.to_string_lossy().as_ref())
                .await;
            Ok(())
        } else {
            debug!("remove_file failed - parent not found");
            Err(FsError::EntityNotFound)
        }
    }

    async fn read_metadata(&self, path: String) -> FsResult<api::Metadata> {
        if let Ok(Some(file)) = self.accessor.search(&self.context, path.as_str()).await {
            FsResult::Ok(super::conv_meta(file))
        } else {
            debug!("read_metadata failed - not found ({})", path);
            FsResult::Err(FsError::EntityNotFound)
        }
    }

    async fn read_symlink_metadata(&self, path: String) -> FsResult<api::Metadata> {
        if let Ok(Some(file)) = self.accessor.search(&self.context, path.as_str()).await {
            FsResult::Ok(super::conv_meta(file))
        } else {
            debug!("read_symlink_metadata failed - not found ({})", path);
            FsResult::Err(FsError::EntityNotFound)
        }
    }

    async fn open(
        &self,
        path: String,
        options: api::OpenOptions,
    ) -> Result<Arc<dyn api::OpenedFile + Send + Sync + 'static>, CallError> {
        // Determine all the flags
        let mut flags = 0i32;
        if options.append {
            flags |= O_APPEND;
        }
        if options.create {
            flags |= O_CREAT;
        }
        if options.create_new {
            flags |= O_CREAT | O_TRUNC;
        }
        if options.read && options.write {
            flags |= O_RDWR;
        } else if options.read {
            flags |= O_RDONLY;
        } else if options.write {
            flags |= O_WRONLY;
        }
        if options.truncate {
            flags |= O_TRUNC;
        }
        let append = options.append;
        let create = options.create | options.create_new;

        // Open the file!
        let handle =
            if let Ok(Some(file)) = self.accessor.search(&self.context, path.as_str()).await {
                match self
                    .accessor
                    .open(&self.context, file.ino, flags as u32)
                    .await
                {
                    Ok(a) => Ok(a),
                    Err(err) => {
                        debug!("open failed (path={}) - {}", path, err);
                        Err(conv_err(err))
                    }
                }
            } else if create == true {
                let path = std::path::Path::new(&path);
                let name = path.file_name().ok_or_else(|| FsError::InvalidInput);
                match name {
                    Ok(name) => {
                        let parent = path.parent().ok_or_else(|| FsError::InvalidInput);
                        match parent {
                            Ok(parent) => {
                                if let Ok(Some(parent)) = self
                                    .accessor
                                    .search(&self.context, parent.to_string_lossy().as_ref())
                                    .await
                                {
                                    match self
                                        .accessor
                                        .create(
                                            &self.context,
                                            parent.ino,
                                            name.to_string_lossy().as_ref(),
                                            0o666 as u32,
                                        )
                                        .await
                                    {
                                        Ok(a) => Ok(a),
                                        Err(err) => {
                                            debug!(
                                                "open failed (path={}) - {}",
                                                path.to_string_lossy(),
                                                err
                                            );
                                            Err(conv_err(err))
                                        }
                                    }
                                } else {
                                    debug!(
                                        "open failed - parent not found ({})",
                                        parent.to_string_lossy()
                                    );
                                    Err(FsError::EntityNotFound)
                                }
                            }
                            Err(err) => {
                                debug!(
                                    "open failed failed - invalid input ({})",
                                    path.to_string_lossy()
                                );
                                Err(err)
                            }
                        }
                    }
                    Err(err) => {
                        debug!("open failed - invalid input ({})", path.to_string_lossy());
                        Err(err)
                    }
                }
            } else {
                debug!("open failed - not found ({})", path);
                Err(FsError::EntityNotFound)
            };

        // Every file as an offset pointer
        // If we are in append mode then we need to change the offeset
        let offset = if let Ok(handle) = &handle {
            if append {
                Arc::new(Mutex::new(handle.spec.size() as u64))
            } else {
                Arc::new(Mutex::new(0u64))
            }
        } else {
            Arc::new(Mutex::new(0u64))
        };

        // Return an opened file
        Ok(Arc::new(OpenedFile::new(
            handle,
            offset,
            self.context.clone(),
            append,
            path,
            self.accessor.clone(),
        )))
    }
}

impl Drop for FileSystem {
    fn drop(&mut self) {
        info!("file system closed - {}", self.accessor.chain.key())
    }
}
