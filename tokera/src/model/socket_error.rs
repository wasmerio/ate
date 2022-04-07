use std::error::Error;
use std::io;
use serde::*;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum SocketErrorKind {
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

impl SocketErrorKind
{
    pub(crate) fn as_str(&self) -> &'static str {
        use SocketErrorKind::*;
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
for SocketErrorKind
{
    fn from(kind: io::ErrorKind) -> SocketErrorKind {
        match kind {
            io::ErrorKind::NotFound => SocketErrorKind::NotFound,
            io::ErrorKind::PermissionDenied => SocketErrorKind::PermissionDenied,
            io::ErrorKind::ConnectionRefused => SocketErrorKind::ConnectionRefused,
            io::ErrorKind::ConnectionReset => SocketErrorKind::ConnectionReset,
            io::ErrorKind::ConnectionAborted => SocketErrorKind::ConnectionAborted,
            io::ErrorKind::NotConnected => SocketErrorKind::NotConnected,
            io::ErrorKind::AddrInUse => SocketErrorKind::AddrInUse,
            io::ErrorKind::AddrNotAvailable => SocketErrorKind::AddrNotAvailable,
            io::ErrorKind::BrokenPipe => SocketErrorKind::BrokenPipe,
            io::ErrorKind::AlreadyExists => SocketErrorKind::AlreadyExists,
            io::ErrorKind::WouldBlock => SocketErrorKind::WouldBlock,
            io::ErrorKind::InvalidInput => SocketErrorKind::InvalidInput,
            io::ErrorKind::InvalidData => SocketErrorKind::InvalidData,
            io::ErrorKind::TimedOut => SocketErrorKind::TimedOut,
            io::ErrorKind::WriteZero => SocketErrorKind::WriteZero,
            io::ErrorKind::Interrupted => SocketErrorKind::Interrupted,
            io::ErrorKind::Unsupported => SocketErrorKind::Unsupported,
            io::ErrorKind::UnexpectedEof => SocketErrorKind::UnexpectedEof,
            io::ErrorKind::OutOfMemory => SocketErrorKind::OutOfMemory,
            io::ErrorKind::Other => SocketErrorKind::Other,
            _ => SocketErrorKind::Other,
        }
    }
}

impl Into<SocketError>
for SocketErrorKind
{
    fn into(self) -> SocketError {
        SocketError::Simple(self)
    }
}

impl Into<io::ErrorKind>
for SocketErrorKind
{
    fn into(self) -> io::ErrorKind {
        match self {
            SocketErrorKind::NotFound => io::ErrorKind::NotFound,
            SocketErrorKind::PermissionDenied => io::ErrorKind::PermissionDenied,
            SocketErrorKind::ConnectionRefused => io::ErrorKind::ConnectionRefused,
            SocketErrorKind::ConnectionReset => io::ErrorKind::ConnectionReset,
            SocketErrorKind::ConnectionAborted => io::ErrorKind::ConnectionAborted,
            SocketErrorKind::NotConnected => io::ErrorKind::NotConnected,
            SocketErrorKind::AddrInUse => io::ErrorKind::AddrInUse,
            SocketErrorKind::AddrNotAvailable => io::ErrorKind::AddrNotAvailable,
            SocketErrorKind::BrokenPipe => io::ErrorKind::BrokenPipe,
            SocketErrorKind::AlreadyExists => io::ErrorKind::AlreadyExists,
            SocketErrorKind::WouldBlock => io::ErrorKind::WouldBlock,
            SocketErrorKind::InvalidInput => io::ErrorKind::InvalidInput,
            SocketErrorKind::InvalidData => io::ErrorKind::InvalidData,
            SocketErrorKind::TimedOut => io::ErrorKind::TimedOut,
            SocketErrorKind::WriteZero => io::ErrorKind::WriteZero,
            SocketErrorKind::Interrupted => io::ErrorKind::Interrupted,
            SocketErrorKind::Unsupported => io::ErrorKind::Unsupported,
            SocketErrorKind::UnexpectedEof => io::ErrorKind::UnexpectedEof,
            SocketErrorKind::OutOfMemory => io::ErrorKind::OutOfMemory,
            SocketErrorKind::Other => io::ErrorKind::Other,
            _ => io::ErrorKind::Other,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum SocketError {
    Os(i32),
    Simple(SocketErrorKind),
    SimpleMessage(SocketErrorKind, String),
}

impl SocketError
{
    pub fn new(kind: SocketErrorKind, msg: String) -> SocketError {
        SocketError::SimpleMessage(kind, msg)
    }
}

impl From<io::Error>
for SocketError
{
    fn from(err: io::Error) -> SocketError {
        if let Some(code) = err.raw_os_error() {
            return SocketError::Os(code);
        }
        #[allow(deprecated)]
        let desc = err.description();
        let kind: SocketErrorKind = err.kind().into();
        if kind.as_str() == desc {
            SocketError::Simple(kind)
        } else {
            SocketError::SimpleMessage(kind, desc.to_string())
        }
    }
}

impl Into<io::Error>
for SocketError
{
    fn into(self) -> io::Error {
        match self {
            SocketError::Os(code) => io::Error::from_raw_os_error(code),
            SocketError::Simple(kind) => {
                let kind: io::ErrorKind = kind.into();
                kind.into()
            },
            SocketError::SimpleMessage(kind, msg) => {
                let kind: io::ErrorKind = kind.into();
                io::Error::new(kind, msg.as_str())
            },
        }
    }
}

impl fmt::Display
for SocketError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SocketError::Os(code) => write!(f, "socket-error(os={})", code),
            SocketError::Simple(kind) => write!(f, "socket-error(kind={})", kind.as_str()),
            SocketError::SimpleMessage(kind, msg) => write!(f, "socket-error(kind={}, msg='{}')", kind.as_str(), msg),
        }
    }
}

pub type SocketResult<T> = std::result::Result<T, SocketError>;