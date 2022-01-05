use crate::wasmer_vfs::FileSystem;
use std::path::Path;
use include_dir::{include_dir, Dir};
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use super::*;

static BIN_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/bin");

pub fn create_root_fs() -> UnionFileSystem {
    let mut mounts = UnionFileSystem::new();
    mounts.mount("root", Path::new("/"), Box::new(TmpFileSystem::default()));
    mounts.create_dir(&Path::new("/bin")).unwrap();
    mounts.create_dir(&Path::new("/dev")).unwrap();
    mounts.create_dir(&Path::new("/etc")).unwrap();
    mounts.create_dir(&Path::new("/tmp")).unwrap();
    mounts.create_dir(&Path::new("/.private")).unwrap();

    append_static_files(&mut mounts);

    let mut os_release = mounts
        .new_open_options()
        .create_new(true)
        .write(true)
        .open("/etc/os-release")
        .unwrap();

    os_release
        .write_all(
            r#"PRETTY_NAME="Tokera WebAssembly Shell 1"
NAME="Tokera/Wasm"
VERSION_ID="1"
VERSION="1"
VERSION_CODENAME=tok
ID=debian
HOME_URL="https://www.tokera.com/"#
                .as_bytes(),
        )
        .unwrap();

    mounts
}

pub fn append_static_files(fs: &mut UnionFileSystem) {
    for file in BIN_DIR.files() {
        if let Some(path) = file.path().to_str() {
            let target = format!("/bin/{}", path);
            
            let mut bin = fs
                .new_open_options()
                .create_new(true)
                .write(true)
                .open(target.as_str())
                .unwrap();

            bin.write_all(file.contents()).unwrap();
        }
    }
}
