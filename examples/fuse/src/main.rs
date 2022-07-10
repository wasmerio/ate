use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use wasmer_bus_fuse::api;
use wasmer_bus_fuse::api::FuseSimplified;
use wasmer_bus_fuse::prelude::*;
use wasmer_bus_fuse::api::FileSystemSimplified;
use wasmer_bus_fuse::api::FileIOSimplified;
use wasmer_bus_fuse::api::OpenedFileSimplified;

#[derive(Debug)]
struct MyFuse { }

#[async_trait]
impl FuseSimplified
for MyFuse {
    async fn mount(&self, _name: String) -> Result<Arc<dyn api::FileSystem>, BusError> {
        Ok(Arc::new(
            MyFileSystem { }
        ))
    }
}

#[derive(Debug)]
struct MyFileSystem { }

static META_DIR: Metadata = Metadata {
    ft: api::FileType {
        dir: true,
        file: false,
        symlink: false,
        char_device: false,
        block_device: false,
        socket: false,
        fifo: false,
    },
    accessed: 0,
    created: 0,
    modified: 0,
    len: README.as_bytes().len() as u64,
};

static META_FILE: Metadata = Metadata {
    ft: api::FileType {
        dir: false,
        file: true,
        symlink: false,
        char_device: false,
        block_device: false,
        socket: false,
        fifo: false,
    },
    accessed: 0,
    created: 0,
    modified: 0,
    len: README.as_bytes().len() as u64,
};

#[async_trait]
impl FileSystemSimplified
for MyFileSystem
{
    async fn init(&self) -> FsResult<()> {
        Ok(())
    }

    async fn read_dir(&self, path: String) -> FsResult<Dir> {
        if path == "/" {
            FsResult::Ok(Dir {
                data: vec![
                    api::DirEntry {
                        path: ".".to_string(),
                        metadata: Some(META_DIR.clone()),
                    },
                    api::DirEntry {
                        path: "readme.md".to_string(),
                        metadata: Some(META_FILE.clone()),
                    },
                ]
            })
        } else {
            FsResult::Err(FsError::EntityNotFound)
        }
    }

    async fn create_dir(&self, _path: String) -> FsResult<Metadata> {
        FsResult::Err(FsError::PermissionDenied)
    }

    async fn remove_dir(&self, _path: String) -> FsResult<()> {
        FsResult::Err(FsError::PermissionDenied)
    }

    async fn rename(&self, _from: String, _to: String) -> FsResult<()> {
        FsResult::Err(FsError::PermissionDenied)
    }

    async fn remove_file(&self, _path: String) -> FsResult<()> {
        FsResult::Err(FsError::PermissionDenied)
    }

    async fn read_metadata(&self, path: String) -> FsResult<Metadata> {
        if path == "/" || path == "." {
            FsResult::Ok(META_DIR.clone())
        } else if path == "/readme.md" {
            FsResult::Ok(META_FILE.clone())
        } else {
            FsResult::Err(FsError::EntityNotFound)
        }
    }

    async fn read_symlink_metadata(&self, path: String) -> FsResult<Metadata> {
        self.read_metadata(path).await
    }

    async fn open(&self, path: String, _options: api::OpenOptions) -> Result<Arc<dyn api::OpenedFile>, BusError> {
        if path == "/readme.md" {
            Result::Ok(Arc::new(MyFile::default()))
        } else {
            Result::Err(BusError::Aborted)
        }
    }
}

static README: &'static str = r#"# Example Readme

This is an example readme file from the fuse example service.
"#;

#[derive(Debug, Default, Clone)]
struct MyFile {
    pos: Arc<AtomicU64>,
}

#[async_trait]
impl OpenedFileSimplified
for MyFile {
    async fn meta(&self) -> FsResult<Metadata> {
        FsResult::Ok(META_FILE.clone())
    }

    async fn unlink(&self) -> FsResult<()> {
        FsResult::Err(FsError::PermissionDenied)
    }

    async fn set_len(&self, _len: u64) -> FsResult<()> {
        FsResult::Err(FsError::PermissionDenied)
    }

    async fn io(&self) -> Result<Arc<dyn api::FileIO>, BusError> {
        Result::Ok(
            Arc::new(self.clone())
        )
    }
}

#[async_trait]
impl FileIOSimplified
for MyFile {
    async fn seek(&self, from: api::SeekFrom) -> FsResult<u64> {
        FsResult::Ok(
            match from {
                api::SeekFrom::Current(a) => {
                    let a = a as u64;
                    self.pos.fetch_add(a, Ordering::AcqRel) + a
                },
                api::SeekFrom::End(a) => {
                    let a = (README.as_bytes().len() as i64) + a;
                    let a = a as u64;
                    self.pos.store(a, Ordering::Release);
                    a
                },
                api::SeekFrom::Start(a) => {
                    let a = a as u64;
                    self.pos.store(a, Ordering::Release);
                    a
                }
            }
        )
    }

    async fn flush(&self) -> FsResult<()> {
        FsResult::Ok(())
    }

    async fn write(&self, _data: Vec<u8>) -> FsResult<u64> {
        FsResult::Err(FsError::PermissionDenied)
    }

    async fn read(&self, len: u64) -> FsResult<Vec<u8>> {
        let buf = README.as_bytes();
        let pos = self.pos.load(Ordering::Acquire) as usize;
        if pos >= buf.len() {
            FsResult::Ok(Vec::new())
        } else {
            let mut pos_end = pos + (len as usize);
            if pos_end > buf.len() {
                pos_end = buf.len();
            }
            FsResult::Ok(README.as_bytes()[pos..pos_end].to_vec())
        }
    }
}

fn main() {
    let fuse = MyFuse { };
    api::FuseService::listen(Arc::new(fuse));
    api::FuseService::serve();
}
