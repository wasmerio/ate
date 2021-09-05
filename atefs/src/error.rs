#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use ate::error::*;

use ate_files::error::FileSystemError;
use ate_files::error::FileSystemErrorKind;
use fuse3::Errno;

pub(crate) fn conv_result<T>(r: std::result::Result<T, FileSystemError>) -> std::result::Result<T, Errno> {
    match r {
        Ok(a) => Ok(a),
        Err(err) => {
            error!("atefs::error {}", err);
            match err {
                FileSystemError(FileSystemErrorKind::NoAccess, _) => Err(libc::EACCES.into()),
                FileSystemError(FileSystemErrorKind::MissingPermissions, _) => Err(libc::EPERM.into()),
                FileSystemError(FileSystemErrorKind::ReadOnly, _) => Err(libc::EPERM.into()),
                FileSystemError(FileSystemErrorKind::InvalidArguments, _) => Err(libc::EINVAL.into()),
                FileSystemError(FileSystemErrorKind::NoEntry, _) => Err(libc::ENOENT.into()),
                FileSystemError(FileSystemErrorKind::DoesNotExist, _) => Err(libc::ENOENT.into()),
                FileSystemError(FileSystemErrorKind::AlreadyExists, _) => Err(libc::EEXIST.into()),
                FileSystemError(FileSystemErrorKind::NotDirectory, _) => Err(libc::ENOTDIR.into()),
                FileSystemError(FileSystemErrorKind::IsDirectory, _) => Err(libc::EISDIR.into()),
                FileSystemError(FileSystemErrorKind::NotImplemented, _) => Err(libc::ENOSYS.into()),
                FileSystemError(FileSystemErrorKind::CommitError(CommitErrorKind::CommsError(CommsErrorKind::Disconnected)), _) => Err(libc::EBUSY.into()),
                FileSystemError(FileSystemErrorKind::CommitError(CommitErrorKind::CommsError(CommsErrorKind::ReadOnly)), _) => Err(libc::EPERM.into()),
                FileSystemError(FileSystemErrorKind::CommitError(CommitErrorKind::ReadOnly), _) => Err(libc::EPERM.into()),
                FileSystemError(FileSystemErrorKind::LoadError(LoadErrorKind::NotFound(_)), _) => Err(libc::ENOENT.into()),
                FileSystemError(FileSystemErrorKind::LoadError(LoadErrorKind::TransformationError(TransformErrorKind::MissingReadKey(_))), _) => Err(libc::EACCES.into()),
                _ => Err(libc::EIO.into())
            }
        }
    }
}