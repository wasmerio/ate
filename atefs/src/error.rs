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
                FileSystemError(FileSystemErrorKind::PermissionDenied, _) => Err(libc::EPERM.into()),
                FileSystemError(FileSystemErrorKind::ReadOnly, _) => Err(libc::EPERM.into()),
                FileSystemError(FileSystemErrorKind::InvalidArguments, _) => Err(libc::EINVAL.into()),
                FileSystemError(FileSystemErrorKind::NoEntry, _) => Err(libc::ENOENT.into()),
                FileSystemError(FileSystemErrorKind::DoesNotExist, _) => Err(libc::ENOENT.into()),
                FileSystemError(FileSystemErrorKind::AlreadyExists, _) => Err(libc::EEXIST.into()),
                FileSystemError(FileSystemErrorKind::NotDirectory, _) => Err(libc::ENOTDIR.into()),
                FileSystemError(FileSystemErrorKind::IsDirectory, _) => Err(libc::EISDIR.into()),
                FileSystemError(FileSystemErrorKind::NotImplemented, _) => Err(libc::ENOSYS.into()),
                FileSystemError(FileSystemErrorKind::AteError(AteErrorKind::CommitError(CommitErrorKind::CommsError(CommsErrorKind::Disconnected))), _) => Err(libc::EBUSY.into()),
                FileSystemError(FileSystemErrorKind::AteError(AteErrorKind::CommsError(CommsErrorKind::Disconnected)), _) => Err(libc::EBUSY.into()),
                _ => Err(libc::EIO.into())
            }
        }
    }
}