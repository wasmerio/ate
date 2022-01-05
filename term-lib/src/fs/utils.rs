use crate::wasmer_vfs::FileSystem;
use include_dir::{include_dir, Dir};
use std::path::Path;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use super::*;

static STATIC_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/static");

pub fn create_root_fs() -> UnionFileSystem {
    let mut mounts = UnionFileSystem::new();
    mounts.mount("root", Path::new("/"), Box::new(TmpFileSystem::default()));
    append_static_dir(&mut mounts, &STATIC_DIR);
    mounts
}

pub fn append_static_dir(fs: &mut UnionFileSystem, dir: &Dir) {
    for dir in dir.dirs() {
        if let Some(path) = dir.path().to_str() {
            let path = format!("/{}", path);
            let path = Path::new(path.as_str());
            if fs.create_dir(path).is_ok() {
                append_static_dir(fs, dir);
            }
        }
    }
    for file in dir.files() {
        if let Some(filename) = file.path().file_name() {
            if filename.to_str() == Some(".marker") {
                continue;
            }
        }
        if let Some(path) = file.path().to_str() {
            let path = format!("/{}", path);

            let mut bin = fs
                .new_open_options()
                .create_new(true)
                .write(true)
                .open(path.as_str())
                .unwrap();

            bin.write_all(file.contents()).unwrap();
        }
    }
}
