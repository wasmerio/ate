use std::error::Error;
use std::io;
use serde::*;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum MioErrorKind {
    NotFound,
    PermissionDenied,
    ConnectionRefused,
    ConnectionReset,
    HostUnreachable,
    NetworkUnreachable,
    ConnectionAborted,
    NotConnected,
    AddrInUse,
    AddrNotAvailable,
    NetworkDown,
    BrokenPipe,
    AlreadyExists,
    WouldBlock,
    NotADirectory,
    IsADirectory,
    DirectoryNotEmpty,
    ReadOnlyFilesystem,
    FilesystemLoop,
    StaleNetworkFileHandle,
    InvalidInput,
    InvalidData,
    TimedOut,
    WriteZero,
    StorageFull,
    NotSeekable,
    FilesystemQuotaExceeded,
    FileTooLarge,
    ResourceBusy,
    ExecutableFileBusy,
    Deadlock,
    CrossesDevices,
    TooManyLinks,
    FilenameTooLong,
    ArgumentListTooLong,
    Interrupted,
    Unsupported,
    UnexpectedEof,
    OutOfMemory,
    Other,
    Uncategorized,
}

impl MioErrorKind
{
    pub(crate) fn as_str(&self) -> &'static str {
        use MioErrorKind::*;
        match *self {
            AddrInUse => "address in use",
            AddrNotAvailable => "address not available",
            AlreadyExists => "entity already exists",
            ArgumentListTooLong => "argument list too long",
            BrokenPipe => "broken pipe",
            ConnectionAborted => "connection aborted",
            ConnectionRefused => "connection refused",
            ConnectionReset => "connection reset",
            CrossesDevices => "cross-device link or rename",
            Deadlock => "deadlock",
            DirectoryNotEmpty => "directory not empty",
            ExecutableFileBusy => "executable file busy",
            FileTooLarge => "file too large",
            FilenameTooLong => "filename too long",
            FilesystemLoop => "filesystem loop or indirection limit (e.g. symlink loop)",
            FilesystemQuotaExceeded => "filesystem quota exceeded",
            HostUnreachable => "host unreachable",
            Interrupted => "operation interrupted",
            InvalidData => "invalid data",
            InvalidInput => "invalid input parameter",
            IsADirectory => "is a directory",
            NetworkDown => "network down",
            NetworkUnreachable => "network unreachable",
            NotADirectory => "not a directory",
            NotConnected => "not connected",
            NotFound => "entity not found",
            NotSeekable => "seek on unseekable file",
            Other => "other error",
            OutOfMemory => "out of memory",
            PermissionDenied => "permission denied",
            ReadOnlyFilesystem => "read-only filesystem or storage medium",
            ResourceBusy => "resource busy",
            StaleNetworkFileHandle => "stale network file handle",
            StorageFull => "no storage space",
            TimedOut => "timed out",
            TooManyLinks => "too many links",
            Uncategorized => "uncategorized error",
            UnexpectedEof => "unexpected end of file",
            Unsupported => "unsupported",
            WouldBlock => "operation would block",
            WriteZero => "write zero",
        }
    }
}

impl From<io::ErrorKind>
for MioErrorKind
{
    fn from(kind: io::ErrorKind) -> MioErrorKind {
        match kind {
            io::ErrorKind::NotFound => MioErrorKind::NotFound,
            io::ErrorKind::PermissionDenied => MioErrorKind::PermissionDenied,
            io::ErrorKind::ConnectionRefused => MioErrorKind::ConnectionRefused,
            io::ErrorKind::ConnectionReset => MioErrorKind::ConnectionReset,
            io::ErrorKind::ConnectionAborted => MioErrorKind::ConnectionAborted,
            io::ErrorKind::NotConnected => MioErrorKind::NotConnected,
            io::ErrorKind::AddrInUse => MioErrorKind::AddrInUse,
            io::ErrorKind::AddrNotAvailable => MioErrorKind::AddrNotAvailable,
            io::ErrorKind::BrokenPipe => MioErrorKind::BrokenPipe,
            io::ErrorKind::AlreadyExists => MioErrorKind::AlreadyExists,
            io::ErrorKind::WouldBlock => MioErrorKind::WouldBlock,
            io::ErrorKind::InvalidInput => MioErrorKind::InvalidInput,
            io::ErrorKind::InvalidData => MioErrorKind::InvalidData,
            io::ErrorKind::TimedOut => MioErrorKind::TimedOut,
            io::ErrorKind::WriteZero => MioErrorKind::WriteZero,
            io::ErrorKind::Interrupted => MioErrorKind::Interrupted,
            io::ErrorKind::Unsupported => MioErrorKind::Unsupported,
            io::ErrorKind::UnexpectedEof => MioErrorKind::UnexpectedEof,
            io::ErrorKind::OutOfMemory => MioErrorKind::OutOfMemory,
            io::ErrorKind::Other => MioErrorKind::Other,
            _ => MioErrorKind::Other,
        }
    }
}

impl Into<io::ErrorKind>
for MioErrorKind
{
    fn into(self) -> io::ErrorKind {
        match self {
            MioErrorKind::NotFound => io::ErrorKind::NotFound,
            MioErrorKind::PermissionDenied => io::ErrorKind::PermissionDenied,
            MioErrorKind::ConnectionRefused => io::ErrorKind::ConnectionRefused,
            MioErrorKind::ConnectionReset => io::ErrorKind::ConnectionReset,
            MioErrorKind::ConnectionAborted => io::ErrorKind::ConnectionAborted,
            MioErrorKind::NotConnected => io::ErrorKind::NotConnected,
            MioErrorKind::AddrInUse => io::ErrorKind::AddrInUse,
            MioErrorKind::AddrNotAvailable => io::ErrorKind::AddrNotAvailable,
            MioErrorKind::BrokenPipe => io::ErrorKind::BrokenPipe,
            MioErrorKind::AlreadyExists => io::ErrorKind::AlreadyExists,
            MioErrorKind::WouldBlock => io::ErrorKind::WouldBlock,
            MioErrorKind::InvalidInput => io::ErrorKind::InvalidInput,
            MioErrorKind::InvalidData => io::ErrorKind::InvalidData,
            MioErrorKind::TimedOut => io::ErrorKind::TimedOut,
            MioErrorKind::WriteZero => io::ErrorKind::WriteZero,
            MioErrorKind::Interrupted => io::ErrorKind::Interrupted,
            MioErrorKind::Unsupported => io::ErrorKind::Unsupported,
            MioErrorKind::UnexpectedEof => io::ErrorKind::UnexpectedEof,
            MioErrorKind::OutOfMemory => io::ErrorKind::OutOfMemory,
            MioErrorKind::Other => io::ErrorKind::Other,
            _ => io::ErrorKind::Other,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum MioError {
    Os(i32),
    Simple(MioErrorKind),
    SimpleMessage(MioErrorKind, String),
}

impl MioError
{
    pub fn new(kind: MioErrorKind, msg: String) -> MioError {
        MioError::SimpleMessage(kind, msg)
    }
}

impl From<io::Error>
for MioError
{
    fn from(err: io::Error) -> MioError {
        if let Some(code) = err.raw_os_error() {
            return MioError::Os(code);
        }
        #[allow(deprecated)]
        let desc = err.description();
        let kind: MioErrorKind = err.kind().into();
        if kind.as_str() == desc {
            MioError::Simple(kind)
        } else {
            MioError::SimpleMessage(kind, desc.to_string())
        }
    }
}

impl Into<io::Error>
for MioError
{
    fn into(self) -> io::Error {
        match self {
            MioError::Os(code) => io::Error::from_raw_os_error(code),
            MioError::Simple(kind) => {
                let kind: io::ErrorKind = kind.into();
                kind.into()
            },
            MioError::SimpleMessage(kind, msg) => {
                let kind: io::ErrorKind = kind.into();
                io::Error::new(kind, msg.as_str())
            },
        }
    }
}

pub type MioResult<T> = std::result::Result<T, MioError>;