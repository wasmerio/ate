use serde::*;
use std::io;
use std::sync::Arc;
use wasm_bus::macros::*;

#[wasm_bus(format = "json")]
pub trait Fuse {
    async fn mount(&self, name: String) -> Arc<dyn FileSystem>;
}

#[wasm_bus(format = "json")]
pub trait FileSystem {
    async fn init(&self) -> FsResult<()>;
    async fn read_dir(&self, path: String) -> FsResult<Dir>;
    async fn create_dir(&self, path: String) -> FsResult<Metadata>;
    async fn remove_dir(&self, path: String) -> FsResult<()>;
    async fn rename(&self, from: String, to: String) -> FsResult<()>;
    async fn remove_file(&self, path: String) -> FsResult<()>;
    async fn read_metadata(&self, path: String) -> FsResult<Metadata>;
    async fn read_symlink_metadata(&self, path: String) -> FsResult<Metadata>;
    async fn open(&self, path: String, options: OpenOptions) -> Arc<dyn OpenedFile>;
}

#[wasm_bus(format = "json")]
pub trait OpenedFile {
    async fn meta(&self) -> FsResult<Metadata>;
    async fn unlink(&self) -> FsResult<()>;
    async fn set_len(&self, len: u64) -> FsResult<()>;
    async fn io(&self) -> Arc<dyn FileIO>;
}

#[wasm_bus(format = "bincode")]
pub trait FileIO {
    async fn seek(&self, from: SeekFrom) -> FsResult<u64>;
    async fn flush(&self) -> FsResult<()>;
    async fn write(&self, data: Vec<u8>) -> FsResult<u64>;
    async fn read(&self, len: u64) -> FsResult<Vec<u8>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenOptions {
    pub read: bool,
    pub write: bool,
    pub create_new: bool,
    pub create: bool,
    pub append: bool,
    pub truncate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SeekFrom {
    Start(u64),
    End(i64),
    Current(i64),
}

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

impl std::fmt::Display for FsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FsError::BaseNotDirectory => write!(f, "base is not a directory"),
            FsError::NotAFile => write!(f, "not a file"),
            FsError::InvalidFd => write!(f, "invalid file descriptor"),
            FsError::AlreadyExists => write!(f, "alreadt existed"),
            FsError::Lock => write!(f, "lock failed"),
            FsError::IOError => write!(f, "fs io error"),
            FsError::AddressInUse => write!(f, "address in use"),
            FsError::AddressNotAvailable => write!(f, "address not available"),
            FsError::BrokenPipe => write!(f, "pipe is broken"),
            FsError::ConnectionAborted => write!(f, "connection aborted"),
            FsError::ConnectionRefused => write!(f, "connection refused"),
            FsError::ConnectionReset => write!(f, "connection reset"),
            FsError::Interrupted => write!(f, "interrupted"),
            FsError::InvalidData => write!(f, "invalid data"),
            FsError::InvalidInput => write!(f, "invalid input"),
            FsError::NotConnected => write!(f, "not connected"),
            FsError::EntityNotFound => write!(f, "entity not found"),
            FsError::NoDevice => write!(f, "no device"),
            FsError::PermissionDenied => write!(f, "permission denied"),
            FsError::TimedOut => write!(f, "timeout has elapsed"),
            FsError::UnexpectedEof => write!(f, "unexpected eof of file"),
            FsError::WouldBlock => write!(f, "call would block"),
            FsError::WriteZero => write!(f, "write zero"),
            FsError::DirectoryNotEmpty => write!(f, "directory is not empty"),
            FsError::UnknownError => write!(f, "unknown error"),
        }
    }
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

pub type FsResult<T> = Result<T, FsError>;
