mod file_io;
mod file_system;
mod fuse;
mod main;
mod opened_file;

pub use main::*;

use wasm_bus_fuse::api;

fn conv_file_type(kind: ate_files::api::FileKind) -> api::FileType {
    let mut ret = api::FileType::default();
    match kind {
        ate_files::api::FileKind::Directory => {
            ret.dir = true;
        }
        ate_files::api::FileKind::RegularFile => {
            ret.file = true;
        }
        ate_files::api::FileKind::FixedFile => {
            ret.file = true;
        }
        ate_files::api::FileKind::SymLink => {
            ret.symlink = true;
        }
    }
    ret
}

fn conv_meta(file: ate_files::attr::FileAttr) -> api::Metadata {
    api::Metadata {
        ft: conv_file_type(file.kind),
        accessed: file.accessed,
        created: file.created,
        modified: file.updated,
        len: file.size,
    }
}
