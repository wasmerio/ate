use serde::*;
use std::io;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FsError {
    BaseNotDirectory,
    NotAFile,
    InvalidFd,
    AlreadyExists,
    Lock,
    IOError,
    AddressInUse,
    AddressNotAvailable,
    BrokenPipe,
    ConnectionAborted,
    ConnectionRefused,
    ConnectionReset,
    Interrupted,
    InvalidData,
    InvalidInput,
    NotConnected,
    EntityNotFound,
    NoDevice,
    PermissionDenied,
    TimedOut,
    UnexpectedEof,
    WouldBlock,
    WriteZero,
    DirectoryNotEmpty,
    UnknownError,
}

impl From<io::Error> for FsError {
    fn from(io_error: io::Error) -> Self {
        match io_error.kind() {
            io::ErrorKind::AddrInUse => FsError::AddressInUse,
            io::ErrorKind::AddrNotAvailable => FsError::AddressNotAvailable,
            io::ErrorKind::AlreadyExists => FsError::AlreadyExists,
            io::ErrorKind::BrokenPipe => FsError::BrokenPipe,
            io::ErrorKind::ConnectionAborted => FsError::ConnectionAborted,
            io::ErrorKind::ConnectionRefused => FsError::ConnectionRefused,
            io::ErrorKind::ConnectionReset => FsError::ConnectionReset,
            io::ErrorKind::Interrupted => FsError::Interrupted,
            io::ErrorKind::InvalidData => FsError::InvalidData,
            io::ErrorKind::InvalidInput => FsError::InvalidInput,
            io::ErrorKind::NotConnected => FsError::NotConnected,
            io::ErrorKind::NotFound => FsError::EntityNotFound,
            io::ErrorKind::PermissionDenied => FsError::PermissionDenied,
            io::ErrorKind::TimedOut => FsError::TimedOut,
            io::ErrorKind::UnexpectedEof => FsError::UnexpectedEof,
            io::ErrorKind::WouldBlock => FsError::WouldBlock,
            io::ErrorKind::WriteZero => FsError::WriteZero,
            io::ErrorKind::Other => FsError::IOError,
            _ => FsError::UnknownError,
        }
    }
}

impl Into<io::ErrorKind> for FsError {
    fn into(self) -> io::ErrorKind {
        match self {
            FsError::AddressInUse => io::ErrorKind::AddrInUse,
            FsError::AddressNotAvailable => io::ErrorKind::AddrNotAvailable,
            FsError::AlreadyExists => io::ErrorKind::AlreadyExists,
            FsError::BrokenPipe => io::ErrorKind::BrokenPipe,
            FsError::ConnectionAborted => io::ErrorKind::ConnectionAborted,
            FsError::ConnectionRefused => io::ErrorKind::ConnectionRefused,
            FsError::ConnectionReset => io::ErrorKind::ConnectionReset,
            FsError::Interrupted => io::ErrorKind::Interrupted,
            FsError::InvalidData => io::ErrorKind::InvalidData,
            FsError::InvalidInput => io::ErrorKind::InvalidInput,
            FsError::NotConnected => io::ErrorKind::NotConnected,
            FsError::EntityNotFound => io::ErrorKind::NotFound,
            FsError::PermissionDenied => io::ErrorKind::PermissionDenied,
            FsError::TimedOut => io::ErrorKind::TimedOut,
            FsError::UnexpectedEof => io::ErrorKind::UnexpectedEof,
            FsError::WouldBlock => io::ErrorKind::WouldBlock,
            FsError::WriteZero => io::ErrorKind::WriteZero,
            FsError::IOError => io::ErrorKind::Other,
            _ => io::ErrorKind::Other,
        }
    }
}

impl Into<io::Error> for FsError {
    fn into(self) -> io::Error {
        let kind: io::ErrorKind = self.into();
        kind.into()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mount {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unmount {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadDir {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDir {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveDir {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rename {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveFile {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Open {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Close {
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewOpen {
    pub read: bool,
    pub write: bool,
    pub create_new: bool,
    pub create: bool,
    pub append: bool,
    pub truncate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unlink {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetLength {
    pub len: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Seek {
    Start(u64),
    End(i64),
    Current(i64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Write {
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Read {
    pub len: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flush {}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileType {
    pub dir: bool,
    pub file: bool,
    pub symlink: bool,
    pub char_device: bool,
    pub block_device: bool,
    pub socket: bool,
    pub fifo: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadMetadata {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadSymlinkMetadata {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub ft: FileType,
    pub accessed: u64,
    pub created: u64,
    pub modified: u64,
    pub len: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirEntry {
    pub path: String,
    pub metadata: Option<Metadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Dir {
    pub data: Vec<DirEntry>,
}
