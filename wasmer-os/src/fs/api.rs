use std::path::Path;
use std::sync::Arc;

use crate::bus::WasmCallerContext;
use crate::wasmer_vfs::*;

pub trait MountedFileSystem
where
    Self: FileSystem + std::fmt::Debug,
{
    fn set_ctx(&self, ctx: &WasmCallerContext);
}

#[derive(Debug, Clone)]
pub struct StaticMountedFileSystem
{
    inner: Arc<Box<dyn FileSystem + Send + Sync + 'static>>,
}

impl StaticMountedFileSystem
{
    pub fn new(inner: Box<dyn FileSystem + Send + Sync + 'static>) -> Self {
        Self {
            inner: Arc::new(inner)
        }
    }
}

impl MountedFileSystem
for StaticMountedFileSystem
{
    fn set_ctx(&self, _ctx: &WasmCallerContext) {
    }
}

impl FileSystem
for StaticMountedFileSystem
{
    fn read_dir(&self, path: &Path) -> Result<ReadDir> {
        self.inner.read_dir(path)
    }

    fn create_dir(&self, _path: &Path) -> Result<()> {
        Err(FsError::PermissionDenied)
    }

    fn remove_dir(&self, _path: &Path) -> Result<()> {
        Err(FsError::PermissionDenied)
    }

    fn rename(&self, _from: &Path, _to: &Path) -> Result<()> {
        Err(FsError::PermissionDenied)
    }
    
    fn metadata(&self, path: &Path) -> Result<Metadata> {
        self.inner.metadata(path)
    }
    
    fn symlink_metadata(&self, path: &Path) -> Result<Metadata> {
        self.metadata(path)
    }
    
    fn remove_file(&self, _path: &Path) -> Result<()> {
        Err(FsError::PermissionDenied)
    }

    fn new_open_options(&self) -> OpenOptions {
        OpenOptions::new(Box::new(
            StaticMountedFileOpener {
                inner: self.inner.clone()
            }
        ))
    }
}

#[derive(Debug, Clone)]
struct StaticMountedFileOpener {
    inner: Arc<Box<dyn FileSystem + Send + Sync + 'static>>,
}

impl FileOpener
for StaticMountedFileOpener
{
    fn open(
        &mut self,
        path: &Path,
        conf: &OpenOptionsConfig,
    ) -> Result<Box<dyn VirtualFile + Send + Sync + 'static>> {
        if conf.write() | conf.create() | conf.create_new() | conf.truncate() {
            return Err(FsError::PermissionDenied);
        }

        let mut opener = self.inner.new_open_options();
        opener.read(conf.read());
        opener.write(conf.write());
        opener.append(conf.append());
        opener.create(conf.create());
        opener.create_new(conf.create_new());
        opener.truncate(conf.truncate());
        opener.open(path)
    }
}

#[derive(Debug, Clone)]
pub struct SharedMountedFileSystem
{
    inner: Arc<dyn FileSystem + Send + Sync + 'static>,
}

impl SharedMountedFileSystem
{
    pub fn new(inner: Arc<dyn FileSystem + Send + Sync + 'static>) -> Self {
        Self {
            inner,
        }
    }
}

impl MountedFileSystem
for SharedMountedFileSystem
{
    fn set_ctx(&self, _ctx: &WasmCallerContext) {
    }
}

impl FileSystem
for SharedMountedFileSystem
{
    fn read_dir(&self, path: &Path) -> Result<ReadDir> {
        self.inner.read_dir(path)
    }

    fn create_dir(&self, path: &Path) -> Result<()> {
        self.inner.create_dir(path)
    }

    fn remove_dir(&self, path: &Path) -> Result<()> {
        self.inner.remove_dir(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        self.inner.rename(from, to)
    }
    
    fn metadata(&self, path: &Path) -> Result<Metadata> {
        self.inner.metadata(path)
    }
    
    fn symlink_metadata(&self, path: &Path) -> Result<Metadata> {
        self.inner.symlink_metadata(path)
    }
    
    fn remove_file(&self, path: &Path) -> Result<()> {
        self.inner.remove_file(path)
    }

    fn new_open_options(&self) -> OpenOptions {
        OpenOptions::new(Box::new(
            SharedMountedFileOpener {
                inner: self.inner.clone()
            }
        ))
    }
}

#[derive(Debug, Clone)]
struct SharedMountedFileOpener {
    inner: Arc<dyn FileSystem + Send + Sync + 'static>,
}

impl FileOpener
for SharedMountedFileOpener
{
    fn open(
        &mut self,
        path: &Path,
        conf: &OpenOptionsConfig,
    ) -> Result<Box<dyn VirtualFile + Send + Sync + 'static>> {
        let mut opener = self.inner.new_open_options();
        opener.read(conf.read());
        opener.write(conf.write());
        opener.append(conf.append());
        opener.create(conf.create());
        opener.create_new(conf.create_new());
        opener.truncate(conf.truncate());
        opener.open(path)
    }
}
