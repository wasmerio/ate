use std::path::Path;
use wasmer_vfs::FileSystem;

use super::*;

pub fn create_root_fs() -> UnionFileSystem {
    let mut mounts = UnionFileSystem::new();
    mounts.mount("root", Path::new("/"), Box::new(TmpFileSystem::default()));
    mounts.create_dir(&Path::new("/bin")).unwrap();
    mounts.create_dir(&Path::new("/dev")).unwrap();
    mounts.create_dir(&Path::new("/etc")).unwrap();
    mounts.create_dir(&Path::new("/tmp")).unwrap();
    mounts.create_dir(&Path::new("/.private")).unwrap();

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
