#![allow(dead_code)]
#![allow(unused)]
#[allow(unused_imports, dead_code)]
use tracing::{info, error, debug, trace, warn};
use wasmer_wasi::vfs::*;
use std::path::{Path, PathBuf};
use std::borrow::Cow;
use std::sync::Arc;

#[derive(Debug, Clone)]
struct MountPoint
{
    path: String,
    name: String,
    fs: Arc<Box<dyn FileSystem>>,
}

#[derive(Debug, Clone)]
pub struct UnionFileSystem
{
    mounts: Vec<MountPoint>
}

impl UnionFileSystem
{
    pub fn new() -> UnionFileSystem {
        UnionFileSystem {
            mounts: Vec::new(),
        }
    }
}

impl UnionFileSystem
{
    pub fn mount(&mut self, name: &str, path: &Path, fs: Box<dyn FileSystem>) {
        let path = path.to_string_lossy().into_owned();
        self.mounts.push(MountPoint {
            path,
            name: name.to_string(),
            fs: Arc::new(fs),
        });
    }

    pub fn unmount(&mut self, path: &Path) {
        let path = path.to_string_lossy().into_owned();
        self.mounts.retain(|mount| {
            mount.path != path
        });
    }

    fn read_dir_internal(&self, path: &Path) -> Result<ReadDir> {
        let path = path.to_string_lossy();
        for (path, mount) in filter_mounts(&self.mounts, path.as_ref()) {
            if let Ok(ret) = mount.fs.read_dir(Path::new(path)) {
                return Ok(ret);
            }
        }
        Err(FsError::EntityNotFound)
    }
}

impl FileSystem
for UnionFileSystem
{
    fn read_dir(&self, path: &Path) -> Result<ReadDir> {
        debug!("read_dir: path={}", path.display());
        self.read_dir_internal(path)
    }
    fn create_dir(&self, path: &Path) -> Result<()> {
        debug!("create_dir: path={}", path.display());
        if self.read_dir_internal(path).is_ok() {
            //return Err(FsError::AlreadyExists);
            return Ok(())
        }
        let path = path.to_string_lossy();
        for (path, mount) in filter_mounts(&self.mounts, path.as_ref()) {
            if let Ok(ret) = mount.fs.create_dir(Path::new(path)) {
                return Ok(ret);
            }
        }
        Err(FsError::EntityNotFound)
    }
    fn remove_dir(&self, path: &Path) -> Result<()> {
        debug!("remove_dir: path={}", path.display());
        let path = path.to_string_lossy();
        for (path, mount) in filter_mounts(&self.mounts, path.as_ref()) {
            if let Ok(ret) = mount.fs.remove_dir(Path::new(path)) {
                return Ok(ret);
            }
        }
        Err(FsError::EntityNotFound)
    }
    fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        debug!("rename: from={} to={}", from.display(), to.display());
        let from = from.to_string_lossy();
        let to = to.to_string_lossy();
        for (path, mount) in filter_mounts(&self.mounts, from.as_ref()) {
            let to = if to.starts_with(mount.path.as_str()) {
                &to[mount.path.len()..]
            } else {
                return Err(FsError::UnknownError);
            };
            if let Ok(ret) = mount.fs.rename(Path::new(from.as_ref()), Path::new(to)) {
                return Ok(ret);
            }
        }
        Err(FsError::EntityNotFound)
    }
    fn metadata(&self, path: &Path) -> Result<Metadata> {
        debug!("metadata: path={}", path.display());
        let path = path.to_string_lossy();
        for (path, mount) in filter_mounts(&self.mounts, path.as_ref()) {
            if let Ok(ret) = mount.fs.metadata(Path::new(path)) {
                return Ok(ret);
            }
        }
        Err(FsError::EntityNotFound)
    }
    fn symlink_metadata(&self, path: &Path) -> Result<Metadata> {
        debug!("symlink_metadata: path={}", path.display());
        let path = path.to_string_lossy();
        for (path, mount) in filter_mounts(&self.mounts, path.as_ref()) {
            if let Ok(ret) = mount.fs.symlink_metadata(Path::new(path)) {
                return Ok(ret);
            }
        }
        Err(FsError::EntityNotFound)
    }
    fn remove_file(&self, path: &Path) -> Result<()> {
        debug!("remove_file: path={}", path.display());
        let path = path.to_string_lossy();
        for (path, mount) in filter_mounts(&self.mounts, path.as_ref()) {
            if let Ok(ret) = mount.fs.remove_file(Path::new(path)) {
                return Ok(ret);
            }
        }
        Err(FsError::EntityNotFound)
    }
    fn new_open_options(&self) -> OpenOptions {
        let opener = Box::new(UnionFileOpener {
            mounts: self.mounts.clone()
        });
        OpenOptions::new(opener)
    }
}

fn filter_mounts<'a, 'b>(mounts: &'a Vec<MountPoint>, mut path: &'b str) -> impl Iterator<Item = (&'b str, &'a MountPoint)> {
    let mut ret = Vec::new();
    for mount in mounts.iter().rev() {
        if path.starts_with(mount.path.as_str()) || path.starts_with(&mount.path[1..]) {
            let path = if mount.path.ends_with("/") {
                &path[mount.path.len()-1..]
            } else {
                &path[mount.path.len()..]
            };
            ret.push((path, mount))
        }
    }
    ret.into_iter()
}

#[derive(Debug)]
pub struct UnionFileOpener {
    mounts: Vec<MountPoint>
}

impl FileOpener
for UnionFileOpener {
    fn open(&mut self, path: &Path, conf: &OpenOptionsConfig) -> Result<Box<dyn VirtualFile>> {
        debug!("open: path={}", path.display());
        let mut ret_err = FsError::EntityNotFound;
        let path = path.to_string_lossy();
        if conf.create() || conf.create_new() {
            for (path, mount) in filter_mounts(&self.mounts, path.as_ref()) {
                if let Ok(mut ret) = mount.fs.new_open_options().open(path) {
                    if conf.create_new() {
                        ret.unlink();
                        continue;
                    }
                    return Ok(ret);
                }
            }
        }
        for (path, mount) in filter_mounts(&self.mounts, path.as_ref()) {
            match mount.fs.new_open_options().set_options(conf.clone()).open(path) {
                Ok(ret) => { return Ok(ret) }
                Err(err) if ret_err == FsError::EntityNotFound => {
                    ret_err = err;
                }
                _ => { }
            }
        }
        Err(ret_err)
    }
}