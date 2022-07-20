pub mod file_io;
pub mod file_system;
mod fuse;
mod main;
mod server;
pub mod opened_file;

pub use main::*;

use wasmer_bus_fuse::api;

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

use ate_files::error::FileSystemError;
fn conv_err(err: FileSystemError) -> api::FsError {
    use ate_files::error::FileSystemErrorKind;

    match err {
        FileSystemError(FileSystemErrorKind::AlreadyExists, _) => api::FsError::AlreadyExists,
        FileSystemError(FileSystemErrorKind::NotDirectory, _) => api::FsError::BaseNotDirectory,
        FileSystemError(FileSystemErrorKind::IsDirectory, _) => api::FsError::InvalidInput,
        FileSystemError(FileSystemErrorKind::DoesNotExist, _) => api::FsError::EntityNotFound,
        FileSystemError(FileSystemErrorKind::NoAccess, _) => api::FsError::PermissionDenied,
        FileSystemError(FileSystemErrorKind::PermissionDenied, _) => api::FsError::PermissionDenied,
        FileSystemError(FileSystemErrorKind::ReadOnly, _) => api::FsError::PermissionDenied,
        FileSystemError(FileSystemErrorKind::InvalidArguments, _) => api::FsError::InvalidInput,
        FileSystemError(FileSystemErrorKind::NoEntry, _) => api::FsError::EntityNotFound,
        FileSystemError(FileSystemErrorKind::NotImplemented, _) => api::FsError::NoDevice,
        FileSystemError(_, _) => api::FsError::IOError,
    }
}
