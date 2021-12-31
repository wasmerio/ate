use async_trait::async_trait;
use ate_files::prelude::*;
use derivative::*;
use std::sync::Arc;
use std::sync::Mutex;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus_fuse::api;
use wasm_bus_fuse::prelude::*;

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct FileIo {
    #[derivative(Debug = "ignore")]
    handle: Arc<OpenHandle>,
    offset: Arc<Mutex<u64>>,
    append: bool,
}

impl FileIo {
    pub fn new(handle: Arc<OpenHandle>, offset: Arc<Mutex<u64>>, append: bool) -> FileIo {
        FileIo {
            handle,
            offset,
            append,
        }
    }
}

#[async_trait]
impl api::FileIOSimplified for FileIo {
    async fn seek(&self, from: api::SeekFrom) -> FsResult<u64> {
        let file = self.handle.as_ref();
        let mut offset = self.offset.lock().unwrap();
        match from {
            api::SeekFrom::Current(a) if a > 0 => {
                if let Some(a) = offset.checked_add(a.abs() as u64) {
                    *offset = a;
                } else {
                    return Err(FsError::InvalidInput);
                }
            }
            api::SeekFrom::Current(a) if a < 0 => {
                if let Some(a) = offset.checked_sub(a.abs() as u64) {
                    *offset = a;
                } else {
                    return Err(FsError::InvalidInput);
                }
            }
            api::SeekFrom::Current(_) => {}
            api::SeekFrom::End(a) if a > 0 => {
                if let Some(a) = file.spec.size().checked_add(a.abs() as u64) {
                    *offset = a;
                } else {
                    return Err(FsError::InvalidInput);
                }
            }
            api::SeekFrom::End(a) if a < 0 => {
                if let Some(a) = file.spec.size().checked_sub(a.abs() as u64) {
                    *offset = a;
                } else {
                    return Err(FsError::InvalidInput);
                }
            }
            api::SeekFrom::End(_) => {}
            api::SeekFrom::Start(a) => {
                *offset = a;
            }
        }
        Ok(*offset)
    }

    async fn flush(&self) -> FsResult<()> {
        let file = self.handle.as_ref();
        file.spec.commit().await.map_err(|err| {
            debug!("flush failed - {}", err);
            FsError::IOError
        })
    }

    async fn write(&self, data: Vec<u8>) -> FsResult<u64> {
        let file = self.handle.as_ref();
        let offset = {
            let mut offset = self.offset.lock().unwrap();
            if self.append {
                *offset = file.spec.size();
            }
            *offset
        };

        file.spec.write(offset, &data[..]).await.map_err(|err| {
            debug!("write failed - {}", err);
            FsError::IOError
        })
    }

    async fn read(&self, len: u64) -> FsResult<Vec<u8>> {
        let file = self.handle.as_ref();
        let offset = self.offset.lock().unwrap().clone();
        file.spec
            .read(offset, len)
            .await
            .map_err(|err| {
                debug!("read failed - {}", err);
                FsError::IOError
            })
            .map(|a| a.to_vec())
    }
}
