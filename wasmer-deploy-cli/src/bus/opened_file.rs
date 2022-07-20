use super::file_io::*;
use async_trait::async_trait;
use ate_files::prelude::*;
use derivative::*;
use std::sync::Arc;
use std::sync::Mutex;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasmer_bus_fuse::api;
use wasmer_bus_fuse::prelude::*;

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct OpenedFile {
    #[derivative(Debug = "ignore")]
    handle: FsResult<Arc<OpenHandle>>,
    offset: Arc<Mutex<u64>>,
    append: bool,
    path: String,
    context: RequestContext,
    #[derivative(Debug = "ignore")]
    accessor: Arc<FileAccessor>,
}

impl OpenedFile {
    pub fn new(
        file: FsResult<Arc<OpenHandle>>,
        offset: Arc<Mutex<u64>>,
        context: RequestContext,
        append: bool,
        path: String,
        accessor: Arc<FileAccessor>,
    ) -> OpenedFile {
        OpenedFile {
            handle: file,
            offset,
            context,
            append,
            path,
            accessor,
        }
    }

    pub async fn io(&self) -> Result<Arc<FileIo>, BusError> {
        let handle = self.handle.clone().map_err(|_err| BusError::BadRequest)?;
        Ok(Arc::new(FileIo::new(
            handle,
            self.offset.clone(),
            self.append,
        )))
    }
}

#[async_trait]
impl api::OpenedFileSimplified for OpenedFile {
    async fn meta(&self) -> FsResult<api::Metadata> {
        if let Ok(Some(file)) = self
            .accessor
            .search(&self.context, self.path.as_str())
            .await
        {
            FsResult::Ok(super::conv_meta(file))
        } else {
            FsResult::Err(FsError::EntityNotFound)
        }
    }

    async fn unlink(&self) -> FsResult<()> {
        let path = std::path::Path::new(&self.path);
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
            Err(FsError::EntityNotFound)
        }
    }

    async fn set_len(&self, len: u64) -> FsResult<()> {
        let file = self.handle.clone()?;
        if let Ok(_) = file.spec.fallocate(len).await {
            Ok(())
        } else {
            Err(FsError::IOError)
        }
    }

    async fn io(&self) -> Result<Arc<dyn api::FileIO>, BusError> {
        let ret = OpenedFile::io(self).await?;
        Ok(ret)
    }
}
