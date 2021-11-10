use std::collections::VecDeque;
use std::path::*;
use wasmer_wasi::vfs::*;
#[allow(unused_imports, dead_code)]
use tracing::{info, error, debug, trace, warn};

pub trait FileSystemExt<T>
where T: FileSystem
{
    fn search(&self, path: &Path, starts_with: Option<&str>, ends_with: Option<&str>) -> Result<PathBuf>;
}

impl<T> FileSystemExt<T>
for T
where T: FileSystem
{
    fn search(&self, path: &Path, starts_with: Option<&str>, ends_with: Option<&str>) -> Result<PathBuf>
    {
        let mut queue = VecDeque::new();
        queue.push_back(path.to_path_buf());

        while let Some(path) = queue.pop_front() {
            for sub in self.read_dir(path.as_path())?.filter_map(|d| d.ok()) {
                if let Ok(meta) = sub.metadata() {
                    if meta.is_dir() {
                        queue.push_back(sub.path());
                    }
                    if meta.is_file() {
                        if let Some(starts_with) = starts_with {
                            if sub.path().to_string_lossy().starts_with(starts_with) == false {
                                continue;
                            }
                        }
                        if let Some(ends_with) = ends_with {
                            if sub.path().to_string_lossy().ends_with(ends_with) == false {
                                continue;
                            }
                        }
                        return Ok(sub.path());
                    }
                }
            }
        }

        return Err(FsError::EntityNotFound);
    }
}