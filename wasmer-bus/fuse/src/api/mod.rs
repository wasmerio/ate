use serde::*;
use std::io;
use std::sync::Arc;
#[allow(unused_imports)]
use wasmer_bus::macros::*;

#[wasmer_bus(format = "json")]
pub trait Fuse {
    async fn mount(&self, name: String) -> Arc<dyn FileSystem>;
}

#[wasmer_bus(format = "json")]
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

#[wasmer_bus(format = "json")]
pub trait OpenedFile {
    async fn meta(&self) -> FsResult<Metadata>;
    async fn unlink(&self) -> FsResult<()>;
    async fn set_len(&self, len: u64) -> FsResult<()>;
    async fn io(&self) -> Arc<dyn FileIO>;
}

#[wasmer_bus(format = "bincode")]
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

impl Into<Box<dyn std::error::Error>> for FsError {
    fn into(self) -> Box<dyn std::error::Error> {
        let kind: io::ErrorKind = self.into();
        let err: io::Error = kind.into();
        Box::new(err)
    }
}

pub type FsResult<T> = Result<T, FsError>;

/*
#[derive(Debug, Clone, serde :: Serialize, serde :: Deserialize)]
pub struct FuseMountRequest {
    pub name: String,
}
#[wasmer_bus::async_trait]
pub trait Fuse
where
    Self: std::fmt::Debug + Send + Sync,
{
    async fn mount(
        &self,
        name: String,
    ) -> std::result::Result<std::sync::Arc<dyn FileSystem>, wasmer_bus::abi::BusError>;
    fn blocking_mount(
        &self,
        name: String,
    ) -> std::result::Result<std::sync::Arc<dyn FileSystem>, wasmer_bus::abi::BusError>;
    fn as_client(&self) -> Option<FuseClient>;
}
#[wasmer_bus::async_trait]
pub trait FuseSimplified
where
    Self: std::fmt::Debug + Send + Sync,
{
    async fn mount(
        &self,
        name: String,
    ) -> std::result::Result<std::sync::Arc<dyn FileSystem>, wasmer_bus::abi::BusError>;
}
#[wasmer_bus::async_trait]
impl<T> Fuse for T
where
    T: FuseSimplified,
{
    async fn mount(
        &self,
        name: String,
    ) -> std::result::Result<std::sync::Arc<dyn FileSystem>, wasmer_bus::abi::BusError> {
        FuseSimplified::mount(self, name).await
    }
    fn blocking_mount(
        &self,
        name: String,
    ) -> std::result::Result<std::sync::Arc<dyn FileSystem>, wasmer_bus::abi::BusError> {
        wasmer_bus::task::block_on(FuseSimplified::mount(self, name))
    }
    fn as_client(&self) -> Option<FuseClient> {
        None
    }
}
#[derive(Debug, Clone)]
pub struct FuseService {}
impl FuseService {
    #[allow(dead_code)]
    pub(crate) fn attach(
        wasm_me: std::sync::Arc<dyn Fuse>,
        call_handle: wasmer_bus::abi::CallHandle,
    ) {
        {
            let wasm_me = wasm_me.clone();
            let call_handle = call_handle.clone();
            wasmer_bus::task::respond_to(
                call_handle,
                wasmer_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |wasm_handle: wasmer_bus::abi::CallHandle, wasm_req: FuseMountRequest| {
                    let wasm_me = wasm_me.clone();
                    let name = wasm_req.name;
                    async move {
                        match wasm_me.mount(name).await {
                            Ok(svc) => {
                                FileSystemService::attach(svc, wasm_handle);
                                wasmer_bus::abi::RespondActionTyped::<()>::Detach
                            },
                            Err(err) => wasmer_bus::abi::RespondActionTyped::<()>::Fault(err)
                        }
                    }
                },
            );
        }
    }
    pub fn listen(wasm_me: std::sync::Arc<dyn Fuse>) {
        {
            let wasm_me = wasm_me.clone();
            wasmer_bus::task::listen(
                wasmer_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |wasm_handle: wasmer_bus::abi::CallHandle, wasm_req: FuseMountRequest| {
                    let wasm_me = wasm_me.clone();
                    let name = wasm_req.name;
                    async move {
                        match wasm_me.mount(name).await {
                            Ok(svc) => {
                                FileSystemService::attach(svc, wasm_handle);
                                wasmer_bus::abi::ListenActionTyped::<()>::Detach
                            },
                            Err(err) => wasmer_bus::abi::ListenActionTyped::<()>::Fault(err)
                        }
                    }
                },
            );
        }
    }
    pub fn serve() {
        wasmer_bus::task::serve();
    }
}
#[derive(Debug, Clone)]
pub struct FuseClient {
    ctx: wasmer_bus::abi::CallContext,
    task: Option<wasmer_bus::abi::Call>,
    join: Option<wasmer_bus::abi::CallJoin<()>>,
}
impl FuseClient {
    pub fn new(wapm: &str) -> Self {
        Self {
            ctx: wasmer_bus::abi::CallContext::NewBusCall {
                wapm: wapm.to_string().into(),
                instance: None,
            },
            task: None,
            join: None,
        }
    }
    pub fn new_with_instance(wapm: &str, instance: &str, access_token: &str) -> Self {
        Self {
            ctx: wasmer_bus::abi::CallContext::NewBusCall {
                wapm: wapm.to_string().into(),
                instance: Some(wasmer_bus::abi::CallInstance::new(instance, access_token)),
            },
            task: None,
            join: None,
        }
    }
    pub fn attach(handle: wasmer_bus::abi::CallHandle) -> Self {
        let handle = wasmer_bus::abi::CallSmartHandle::new(handle);
        Self {
            ctx: wasmer_bus::abi::CallContext::OwnedSubCall { parent: handle },
            task: None,
            join: None,
        }
    }
    pub fn wait(self) -> Result<(), wasmer_bus::abi::BusError> {
        if let Some(join) = self.join {
            join.wait()?;
        }
        if let Some(task) = self.task {
            task.join()?.wait()?;
        }
        Ok(())
    }
    pub fn try_wait(&mut self) -> Result<Option<()>, wasmer_bus::abi::BusError> {
        if let Some(task) = self.task.take() {
            self.join.replace(task.join()?);
        }
        if let Some(join) = self.join.as_mut() {
            join.try_wait()
        } else {
            Ok(None)
        }
    }
    pub async fn mount(
        &self,
        name: String,
    ) -> std::result::Result<std::sync::Arc<dyn FileSystem>, wasmer_bus::abi::BusError> {
        let request = FuseMountRequest { name };
        let handle = wasmer_bus::abi::call(
            self.ctx.clone(),
            wasmer_bus::abi::SerializationFormat::Json,
            request,
        )
        .detach()?;
        Ok(Arc::new(FileSystemClient::attach(handle)))
    }
    pub fn blocking_mount(
        &self,
        name: String,
    ) -> std::result::Result<std::sync::Arc<dyn FileSystem>, wasmer_bus::abi::BusError> {
        wasmer_bus::task::block_on(self.mount(name))
    }
}
impl std::future::Future for FuseClient {
    type Output = Result<(), wasmer_bus::abi::BusError>;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if let Some(task) = self.task.take() {
            self.join.replace(task.join()?);
        }
        if let Some(join) = self.join.as_mut() {
            let join = std::pin::Pin::new(join);
            return join.poll(cx);
        } else {
            std::task::Poll::Ready(Ok(()))
        }
    }
}
#[wasmer_bus::async_trait]
impl Fuse for FuseClient {
    async fn mount(
        &self,
        name: String,
    ) -> std::result::Result<std::sync::Arc<dyn FileSystem>, wasmer_bus::abi::BusError> {
        FuseClient::mount(self, name).await
    }
    fn blocking_mount(
        &self,
        name: String,
    ) -> std::result::Result<std::sync::Arc<dyn FileSystem>, wasmer_bus::abi::BusError> {
        FuseClient::blocking_mount(self, name)
    }
    fn as_client(&self) -> Option<FuseClient> {
        Some(self.clone())
    }
}

#[derive(Debug, Clone, serde :: Serialize, serde :: Deserialize)]
pub struct FileSystemInitRequest {}
#[derive(Debug, Clone, serde :: Serialize, serde :: Deserialize)]
pub struct FileSystemReadDirRequest {
    pub path: String,
}
#[derive(Debug, Clone, serde :: Serialize, serde :: Deserialize)]
pub struct FileSystemCreateDirRequest {
    pub path: String,
}
#[derive(Debug, Clone, serde :: Serialize, serde :: Deserialize)]
pub struct FileSystemRemoveDirRequest {
    pub path: String,
}
#[derive(Debug, Clone, serde :: Serialize, serde :: Deserialize)]
pub struct FileSystemRenameRequest {
    pub from: String,
    pub to: String,
}
#[derive(Debug, Clone, serde :: Serialize, serde :: Deserialize)]
pub struct FileSystemRemoveFileRequest {
    pub path: String,
}
#[derive(Debug, Clone, serde :: Serialize, serde :: Deserialize)]
pub struct FileSystemReadMetadataRequest {
    pub path: String,
}
#[derive(Debug, Clone, serde :: Serialize, serde :: Deserialize)]
pub struct FileSystemReadSymlinkMetadataRequest {
    pub path: String,
}
#[derive(Debug, Clone, serde :: Serialize, serde :: Deserialize)]
pub struct FileSystemOpenRequest {
    pub path: String,
    pub options: OpenOptions,
}
#[wasmer_bus::async_trait]
pub trait FileSystem
where
    Self: std::fmt::Debug + Send + Sync,
{
    async fn init(&self) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError>;
    async fn read_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Dir>, wasmer_bus::abi::BusError>;
    async fn create_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError>;
    async fn remove_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError>;
    async fn rename(
        &self,
        from: String,
        to: String,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError>;
    async fn remove_file(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError>;
    async fn read_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError>;
    async fn read_symlink_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError>;
    async fn open(
        &self,
        path: String,
        options: OpenOptions,
    ) -> std::result::Result<std::sync::Arc<dyn OpenedFile>, wasmer_bus::abi::BusError>;
    fn blocking_init(&self) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError>;
    fn blocking_read_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Dir>, wasmer_bus::abi::BusError>;
    fn blocking_create_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError>;
    fn blocking_remove_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError>;
    fn blocking_rename(
        &self,
        from: String,
        to: String,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError>;
    fn blocking_remove_file(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError>;
    fn blocking_read_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError>;
    fn blocking_read_symlink_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError>;
    fn blocking_open(
        &self,
        path: String,
        options: OpenOptions,
    ) -> std::result::Result<std::sync::Arc<dyn OpenedFile>, wasmer_bus::abi::BusError>;
    fn as_client(&self) -> Option<FileSystemClient>;
}
#[wasmer_bus::async_trait]
pub trait FileSystemSimplified
where
    Self: std::fmt::Debug + Send + Sync,
{
    async fn init(&self) -> FsResult<()>;
    async fn read_dir(&self, path: String) -> FsResult<Dir>;
    async fn create_dir(&self, path: String) -> FsResult<Metadata>;
    async fn remove_dir(&self, path: String) -> FsResult<()>;
    async fn rename(&self, from: String, to: String) -> FsResult<()>;
    async fn remove_file(&self, path: String) -> FsResult<()>;
    async fn read_metadata(&self, path: String) -> FsResult<Metadata>;
    async fn read_symlink_metadata(&self, path: String) -> FsResult<Metadata>;
    async fn open(
        &self,
        path: String,
        options: OpenOptions,
    ) -> std::result::Result<std::sync::Arc<dyn OpenedFile>, wasmer_bus::abi::BusError>;
}
#[wasmer_bus::async_trait]
impl<T> FileSystem for T
where
    T: FileSystemSimplified,
{
    async fn init(&self) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        Ok(FileSystemSimplified::init(self).await)
    }
    fn blocking_init(&self) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        Ok(wasmer_bus::task::block_on(FileSystemSimplified::init(self)))
    }
    async fn read_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Dir>, wasmer_bus::abi::BusError> {
        Ok(FileSystemSimplified::read_dir(self, path).await)
    }
    fn blocking_read_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Dir>, wasmer_bus::abi::BusError> {
        Ok(wasmer_bus::task::block_on(FileSystemSimplified::read_dir(
            self, path,
        )))
    }
    async fn create_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError> {
        Ok(FileSystemSimplified::create_dir(self, path).await)
    }
    fn blocking_create_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError> {
        Ok(wasmer_bus::task::block_on(FileSystemSimplified::create_dir(
            self, path,
        )))
    }
    async fn remove_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        Ok(FileSystemSimplified::remove_dir(self, path).await)
    }
    fn blocking_remove_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        Ok(wasmer_bus::task::block_on(FileSystemSimplified::remove_dir(
            self, path,
        )))
    }
    async fn rename(
        &self,
        from: String,
        to: String,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        Ok(FileSystemSimplified::rename(self, from, to).await)
    }
    fn blocking_rename(
        &self,
        from: String,
        to: String,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        Ok(wasmer_bus::task::block_on(FileSystemSimplified::rename(
            self, from, to,
        )))
    }
    async fn remove_file(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        Ok(FileSystemSimplified::remove_file(self, path).await)
    }
    fn blocking_remove_file(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        Ok(wasmer_bus::task::block_on(FileSystemSimplified::remove_file(
            self, path,
        )))
    }
    async fn read_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError> {
        Ok(FileSystemSimplified::read_metadata(self, path).await)
    }
    fn blocking_read_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError> {
        Ok(wasmer_bus::task::block_on(
            FileSystemSimplified::read_metadata(self, path),
        ))
    }
    async fn read_symlink_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError> {
        Ok(FileSystemSimplified::read_symlink_metadata(self, path).await)
    }
    fn blocking_read_symlink_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError> {
        Ok(wasmer_bus::task::block_on(
            FileSystemSimplified::read_symlink_metadata(self, path),
        ))
    }
    async fn open(
        &self,
        path: String,
        options: OpenOptions,
    ) -> std::result::Result<std::sync::Arc<dyn OpenedFile>, wasmer_bus::abi::BusError> {
        FileSystemSimplified::open(self, path, options).await
    }
    fn blocking_open(
        &self,
        path: String,
        options: OpenOptions,
    ) -> std::result::Result<std::sync::Arc<dyn OpenedFile>, wasmer_bus::abi::BusError> {
        wasmer_bus::task::block_on(FileSystemSimplified::open(self, path, options))
    }
    fn as_client(&self) -> Option<FileSystemClient> {
        None
    }
}
#[derive(Debug, Clone)]
pub struct FileSystemService {}
impl FileSystemService {
    #[allow(dead_code)]
    pub(crate) fn attach(
        wasm_me: std::sync::Arc<dyn FileSystem>,
        call_handle: wasmer_bus::abi::CallHandle,
    ) {
        {
            let wasm_me = wasm_me.clone();
            let call_handle = call_handle.clone();
            wasmer_bus::task::respond_to(
                call_handle,
                wasmer_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |wasm_handle: wasmer_bus::abi::CallHandle, wasm_req: FileSystemInitRequest| {
                    let wasm_me = wasm_me.clone();
                    async move {
                        match wasm_me.init().await {
                            Ok(res) => wasmer_bus::abi::RespondActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::RespondActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
        {
            let wasm_me = wasm_me.clone();
            let call_handle = call_handle.clone();
            wasmer_bus::task::respond_to(
                call_handle,
                wasmer_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |wasm_handle: wasmer_bus::abi::CallHandle,
                      wasm_req: FileSystemReadDirRequest| {
                    let wasm_me = wasm_me.clone();
                    let path = wasm_req.path;
                    async move {
                        match wasm_me.read_dir(path).await {
                            Ok(res) => wasmer_bus::abi::RespondActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::RespondActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
        {
            let wasm_me = wasm_me.clone();
            let call_handle = call_handle.clone();
            wasmer_bus::task::respond_to(
                call_handle,
                wasmer_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |wasm_handle: wasmer_bus::abi::CallHandle,
                      wasm_req: FileSystemCreateDirRequest| {
                    let wasm_me = wasm_me.clone();
                    let path = wasm_req.path;
                    async move {
                        match wasm_me.create_dir(path).await {
                            Ok(res) => wasmer_bus::abi::RespondActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::RespondActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
        {
            let wasm_me = wasm_me.clone();
            let call_handle = call_handle.clone();
            wasmer_bus::task::respond_to(
                call_handle,
                wasmer_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |wasm_handle: wasmer_bus::abi::CallHandle,
                      wasm_req: FileSystemRemoveDirRequest| {
                    let wasm_me = wasm_me.clone();
                    let path = wasm_req.path;
                    async move {
                        match wasm_me.remove_dir(path).await {
                            Ok(res) => wasmer_bus::abi::RespondActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::RespondActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
        {
            let wasm_me = wasm_me.clone();
            let call_handle = call_handle.clone();
            wasmer_bus::task::respond_to(
                call_handle,
                wasmer_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |wasm_handle: wasmer_bus::abi::CallHandle, wasm_req: FileSystemRenameRequest| {
                    let wasm_me = wasm_me.clone();
                    let from = wasm_req.from;
                    let to = wasm_req.to;
                    async move {
                        match wasm_me.rename(from, to).await {
                            Ok(res) => wasmer_bus::abi::RespondActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::RespondActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
        {
            let wasm_me = wasm_me.clone();
            let call_handle = call_handle.clone();
            wasmer_bus::task::respond_to(
                call_handle,
                wasmer_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |wasm_handle: wasmer_bus::abi::CallHandle,
                      wasm_req: FileSystemRemoveFileRequest| {
                    let wasm_me = wasm_me.clone();
                    let path = wasm_req.path;
                    async move {
                        match wasm_me.remove_file(path).await {
                            Ok(res) => wasmer_bus::abi::RespondActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::RespondActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
        {
            let wasm_me = wasm_me.clone();
            let call_handle = call_handle.clone();
            wasmer_bus::task::respond_to(
                call_handle,
                wasmer_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |wasm_handle: wasmer_bus::abi::CallHandle,
                      wasm_req: FileSystemReadMetadataRequest| {
                    let wasm_me = wasm_me.clone();
                    let path = wasm_req.path;
                    async move {
                        match wasm_me.read_metadata(path).await {
                            Ok(res) => wasmer_bus::abi::RespondActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::RespondActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
        {
            let wasm_me = wasm_me.clone();
            let call_handle = call_handle.clone();
            wasmer_bus::task::respond_to(
                call_handle,
                wasmer_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |wasm_handle: wasmer_bus::abi::CallHandle,
                      wasm_req: FileSystemReadSymlinkMetadataRequest| {
                    let wasm_me = wasm_me.clone();
                    let path = wasm_req.path;
                    async move {
                        match wasm_me.read_symlink_metadata(path).await {
                            Ok(res) => wasmer_bus::abi::RespondActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::RespondActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
        {
            let wasm_me = wasm_me.clone();
            let call_handle = call_handle.clone();
            wasmer_bus::task::respond_to(
                call_handle,
                wasmer_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |wasm_handle: wasmer_bus::abi::CallHandle, wasm_req: FileSystemOpenRequest| {
                    let wasm_me = wasm_me.clone();
                    let path = wasm_req.path;
                    let options = wasm_req.options;
                    async move {
                        match wasm_me.open(path, options).await {
                            Ok(svc) => {
                                OpenedFileService::attach(svc, wasm_handle);
                                wasmer_bus::abi::RespondActionTyped::<()>::Detach
                            },
                            Err(err) => wasmer_bus::abi::RespondActionTyped::<()>::Fault(err)
                        }
                    }
                },
            );
        }
    }
    pub fn listen(wasm_me: std::sync::Arc<dyn FileSystem>) {
        {
            let wasm_me = wasm_me.clone();
            wasmer_bus::task::listen(
                wasmer_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasmer_bus::abi::CallHandle, wasm_req: FileSystemInitRequest| {
                    let wasm_me = wasm_me.clone();
                    async move {
                        match wasm_me.init().await {
                            Ok(res) => wasmer_bus::abi::ListenActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::ListenActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasmer_bus::task::listen(
                wasmer_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasmer_bus::abi::CallHandle,
                      wasm_req: FileSystemReadDirRequest| {
                    let wasm_me = wasm_me.clone();
                    let path = wasm_req.path;
                    async move {
                        match wasm_me.read_dir(path).await {
                            Ok(res) => wasmer_bus::abi::ListenActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::ListenActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasmer_bus::task::listen(
                wasmer_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasmer_bus::abi::CallHandle,
                      wasm_req: FileSystemCreateDirRequest| {
                    let wasm_me = wasm_me.clone();
                    let path = wasm_req.path;
                    async move {
                        match wasm_me.create_dir(path).await {
                            Ok(res) => wasmer_bus::abi::ListenActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::ListenActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasmer_bus::task::listen(
                wasmer_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasmer_bus::abi::CallHandle,
                      wasm_req: FileSystemRemoveDirRequest| {
                    let wasm_me = wasm_me.clone();
                    let path = wasm_req.path;
                    async move {
                        match wasm_me.remove_dir(path).await {
                            Ok(res) => wasmer_bus::abi::ListenActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::ListenActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasmer_bus::task::listen(
                wasmer_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasmer_bus::abi::CallHandle,
                      wasm_req: FileSystemRenameRequest| {
                    let wasm_me = wasm_me.clone();
                    let from = wasm_req.from;
                    let to = wasm_req.to;
                    async move {
                        match wasm_me.rename(from, to).await {
                            Ok(res) => wasmer_bus::abi::ListenActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::ListenActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasmer_bus::task::listen(
                wasmer_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasmer_bus::abi::CallHandle,
                      wasm_req: FileSystemRemoveFileRequest| {
                    let wasm_me = wasm_me.clone();
                    let path = wasm_req.path;
                    async move {
                        match wasm_me.remove_file(path).await {
                            Ok(res) => wasmer_bus::abi::ListenActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::ListenActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasmer_bus::task::listen(
                wasmer_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasmer_bus::abi::CallHandle,
                      wasm_req: FileSystemReadMetadataRequest| {
                    let wasm_me = wasm_me.clone();
                    let path = wasm_req.path;
                    async move {
                        match wasm_me.read_metadata(path).await {
                            Ok(res) => wasmer_bus::abi::ListenActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::ListenActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasmer_bus::task::listen(
                wasmer_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasmer_bus::abi::CallHandle,
                      wasm_req: FileSystemReadSymlinkMetadataRequest| {
                    let wasm_me = wasm_me.clone();
                    let path = wasm_req.path;
                    async move {
                        match wasm_me.read_symlink_metadata(path).await {
                            Ok(res) => wasmer_bus::abi::ListenActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::ListenActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasmer_bus::task::listen(
                wasmer_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |wasm_handle: wasmer_bus::abi::CallHandle, wasm_req: FileSystemOpenRequest| {
                    let wasm_me = wasm_me.clone();
                    let path = wasm_req.path;
                    let options = wasm_req.options;
                    async move {
                        match wasm_me.open(path, options).await {
                            Ok(svc) => {
                                OpenedFileService::attach(svc, wasm_handle);
                                wasmer_bus::abi::ListenActionTyped::<()>::Detach
                            },
                            Err(err) => wasmer_bus::abi::ListenActionTyped::<()>::Fault(err)
                        }
                    }
                },
            );
        }
    }
    pub fn serve() {
        wasmer_bus::task::serve();
    }
}
#[derive(Debug, Clone)]
pub struct FileSystemClient {
    ctx: wasmer_bus::abi::CallContext,
    task: Option<wasmer_bus::abi::Call>,
    join: Option<wasmer_bus::abi::CallJoin<()>>,
}
impl FileSystemClient {
    pub fn new(wapm: &str) -> Self {
        Self {
            ctx: wasmer_bus::abi::CallContext::NewBusCall {
                wapm: wapm.to_string().into(),
                instance: None,
            },
            task: None,
            join: None,
        }
    }
    pub fn new_with_instance(wapm: &str, instance: &str, access_token: &str) -> Self {
        Self {
            ctx: wasmer_bus::abi::CallContext::NewBusCall {
                wapm: wapm.to_string().into(),
                instance: Some(wasmer_bus::abi::CallInstance::new(instance, access_token)),
            },
            task: None,
            join: None,
        }
    }
    pub fn attach(handle: wasmer_bus::abi::CallHandle) -> Self {
        let handle = wasmer_bus::abi::CallSmartHandle::new(handle);
        Self {
            ctx: wasmer_bus::abi::CallContext::OwnedSubCall { parent: handle },
            task: None,
            join: None,
        }
    }
    pub fn wait(self) -> Result<(), wasmer_bus::abi::BusError> {
        if let Some(join) = self.join {
            join.wait()?;
        }
        if let Some(task) = self.task {
            task.join()?.wait()?;
        }
        Ok(())
    }
    pub fn try_wait(&mut self) -> Result<Option<()>, wasmer_bus::abi::BusError> {
        if let Some(task) = self.task.take() {
            self.join.replace(task.join()?);
        }
        if let Some(join) = self.join.as_mut() {
            join.try_wait()
        } else {
            Ok(None)
        }
    }
    pub async fn init(&self) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        let request = FileSystemInitRequest {};
        wasmer_bus::abi::call(
            self.ctx.clone(),
            wasmer_bus::abi::SerializationFormat::Json,
            request,
        )
        .invoke()
        .join()?
        .await
    }
    pub async fn read_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Dir>, wasmer_bus::abi::BusError> {
        let request = FileSystemReadDirRequest { path };
        wasmer_bus::abi::call(
            self.ctx.clone(),
            wasmer_bus::abi::SerializationFormat::Json,
            request,
        )
        .invoke()
        .join()?
        .await
    }
    pub async fn create_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError> {
        let request = FileSystemCreateDirRequest { path };
        wasmer_bus::abi::call(
            self.ctx.clone(),
            wasmer_bus::abi::SerializationFormat::Json,
            request,
        )
        .invoke()
        .join()?
        .await
    }
    pub async fn remove_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        let request = FileSystemRemoveDirRequest { path };
        wasmer_bus::abi::call(
            self.ctx.clone(),
            wasmer_bus::abi::SerializationFormat::Json,
            request,
        )
        .invoke()
        .join()?
        .await
    }
    pub async fn rename(
        &self,
        from: String,
        to: String,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        let request = FileSystemRenameRequest { from, to };
        wasmer_bus::abi::call(
            self.ctx.clone(),
            wasmer_bus::abi::SerializationFormat::Json,
            request,
        )
        .invoke()
        .join()?
        .await
    }
    pub async fn remove_file(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        let request = FileSystemRemoveFileRequest { path };
        wasmer_bus::abi::call(
            self.ctx.clone(),
            wasmer_bus::abi::SerializationFormat::Json,
            request,
        )
        .invoke()
        .join()?
        .await
    }
    pub async fn read_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError> {
        let request = FileSystemReadMetadataRequest { path };
        wasmer_bus::abi::call(
            self.ctx.clone(),
            wasmer_bus::abi::SerializationFormat::Json,
            request,
        )
        .invoke()
        .join()?
        .await
    }
    pub async fn read_symlink_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError> {
        let request = FileSystemReadSymlinkMetadataRequest { path };
        wasmer_bus::abi::call(
            self.ctx.clone(),
            wasmer_bus::abi::SerializationFormat::Json,
            request,
        )
        .invoke()
        .join()?
        .await
    }
    pub async fn open(
        &self,
        path: String,
        options: OpenOptions,
    ) -> std::result::Result<std::sync::Arc<dyn OpenedFile>, wasmer_bus::abi::BusError> {
        let request = FileSystemOpenRequest { path, options };
        let handle = wasmer_bus::abi::call(
            self.ctx.clone(),
            wasmer_bus::abi::SerializationFormat::Json,
            request,
        )
        .detach()?;
        Ok(Arc::new(OpenedFileClient::attach(handle)))
    }
    pub fn blocking_init(&self) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        wasmer_bus::task::block_on(self.init())
    }
    pub fn blocking_read_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Dir>, wasmer_bus::abi::BusError> {
        wasmer_bus::task::block_on(self.read_dir(path))
    }
    pub fn blocking_create_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError> {
        wasmer_bus::task::block_on(self.create_dir(path))
    }
    pub fn blocking_remove_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        wasmer_bus::task::block_on(self.remove_dir(path))
    }
    pub fn blocking_rename(
        &self,
        from: String,
        to: String,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        wasmer_bus::task::block_on(self.rename(from, to))
    }
    pub fn blocking_remove_file(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        wasmer_bus::task::block_on(self.remove_file(path))
    }
    pub fn blocking_read_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError> {
        wasmer_bus::task::block_on(self.read_metadata(path))
    }
    pub fn blocking_read_symlink_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError> {
        wasmer_bus::task::block_on(self.read_symlink_metadata(path))
    }
    pub fn blocking_open(
        &self,
        path: String,
        options: OpenOptions,
    ) -> std::result::Result<std::sync::Arc<dyn OpenedFile>, wasmer_bus::abi::BusError> {
        wasmer_bus::task::block_on(self.open(path, options))
    }
}
impl std::future::Future for FileSystemClient {
    type Output = Result<(), wasmer_bus::abi::BusError>;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if let Some(task) = self.task.take() {
            self.join.replace(task.join()?);
        }
        if let Some(join) = self.join.as_mut() {
            let join = std::pin::Pin::new(join);
            return join.poll(cx);
        } else {
            std::task::Poll::Ready(Ok(()))
        }
    }
}
#[wasmer_bus::async_trait]
impl FileSystem for FileSystemClient {
    async fn init(&self) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        FileSystemClient::init(self).await
    }
    fn blocking_init(&self) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        FileSystemClient::blocking_init(self)
    }
    async fn read_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Dir>, wasmer_bus::abi::BusError> {
        FileSystemClient::read_dir(self, path).await
    }
    fn blocking_read_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Dir>, wasmer_bus::abi::BusError> {
        FileSystemClient::blocking_read_dir(self, path)
    }
    async fn create_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError> {
        FileSystemClient::create_dir(self, path).await
    }
    fn blocking_create_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError> {
        FileSystemClient::blocking_create_dir(self, path)
    }
    async fn remove_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        FileSystemClient::remove_dir(self, path).await
    }
    fn blocking_remove_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        FileSystemClient::blocking_remove_dir(self, path)
    }
    async fn rename(
        &self,
        from: String,
        to: String,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        FileSystemClient::rename(self, from, to).await
    }
    fn blocking_rename(
        &self,
        from: String,
        to: String,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        FileSystemClient::blocking_rename(self, from, to)
    }
    async fn remove_file(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        FileSystemClient::remove_file(self, path).await
    }
    fn blocking_remove_file(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        FileSystemClient::blocking_remove_file(self, path)
    }
    async fn read_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError> {
        FileSystemClient::read_metadata(self, path).await
    }
    fn blocking_read_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError> {
        FileSystemClient::blocking_read_metadata(self, path)
    }
    async fn read_symlink_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError> {
        FileSystemClient::read_symlink_metadata(self, path).await
    }
    fn blocking_read_symlink_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError> {
        FileSystemClient::blocking_read_symlink_metadata(self, path)
    }
    async fn open(
        &self,
        path: String,
        options: OpenOptions,
    ) -> std::result::Result<std::sync::Arc<dyn OpenedFile>, wasmer_bus::abi::BusError> {
        FileSystemClient::open(self, path, options).await
    }
    fn blocking_open(
        &self,
        path: String,
        options: OpenOptions,
    ) -> std::result::Result<std::sync::Arc<dyn OpenedFile>, wasmer_bus::abi::BusError> {
        FileSystemClient::blocking_open(self, path, options)
    }
    fn as_client(&self) -> Option<FileSystemClient> {
        Some(self.clone())
    }
}

#[derive(Debug, Clone, serde :: Serialize, serde :: Deserialize)]
pub struct OpenedFileMetaRequest {}
#[derive(Debug, Clone, serde :: Serialize, serde :: Deserialize)]
pub struct OpenedFileUnlinkRequest {}
#[derive(Debug, Clone, serde :: Serialize, serde :: Deserialize)]
pub struct OpenedFileSetLenRequest {
    pub len: u64,
}
#[derive(Debug, Clone, serde :: Serialize, serde :: Deserialize)]
pub struct OpenedFileIoRequest {}
#[wasmer_bus::async_trait]
pub trait OpenedFile
where
    Self: std::fmt::Debug + Send + Sync,
{
    async fn meta(&self) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError>;
    async fn unlink(&self) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError>;
    async fn set_len(&self, len: u64)
        -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError>;
    async fn io(&self) -> std::result::Result<std::sync::Arc<dyn FileIO>, wasmer_bus::abi::BusError>;
    fn blocking_meta(&self) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError>;
    fn blocking_unlink(&self) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError>;
    fn blocking_set_len(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError>;
    fn blocking_io(
        &self,
    ) -> std::result::Result<std::sync::Arc<dyn FileIO>, wasmer_bus::abi::BusError>;
    fn as_client(&self) -> Option<OpenedFileClient>;
}
#[wasmer_bus::async_trait]
pub trait OpenedFileSimplified
where
    Self: std::fmt::Debug + Send + Sync,
{
    async fn meta(&self) -> FsResult<Metadata>;
    async fn unlink(&self) -> FsResult<()>;
    async fn set_len(&self, len: u64) -> FsResult<()>;
    async fn io(&self) -> std::result::Result<std::sync::Arc<dyn FileIO>, wasmer_bus::abi::BusError>;
}
#[wasmer_bus::async_trait]
impl<T> OpenedFile for T
where
    T: OpenedFileSimplified,
{
    async fn meta(&self) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError> {
        Ok(OpenedFileSimplified::meta(self).await)
    }
    fn blocking_meta(&self) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError> {
        Ok(wasmer_bus::task::block_on(OpenedFileSimplified::meta(self)))
    }
    async fn unlink(&self) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        Ok(OpenedFileSimplified::unlink(self).await)
    }
    fn blocking_unlink(&self) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        Ok(wasmer_bus::task::block_on(OpenedFileSimplified::unlink(self)))
    }
    async fn set_len(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        Ok(OpenedFileSimplified::set_len(self, len).await)
    }
    fn blocking_set_len(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        Ok(wasmer_bus::task::block_on(OpenedFileSimplified::set_len(
            self, len,
        )))
    }
    async fn io(&self) -> std::result::Result<std::sync::Arc<dyn FileIO>, wasmer_bus::abi::BusError> {
        OpenedFileSimplified::io(self).await
    }
    fn blocking_io(
        &self,
    ) -> std::result::Result<std::sync::Arc<dyn FileIO>, wasmer_bus::abi::BusError> {
        wasmer_bus::task::block_on(OpenedFileSimplified::io(self))
    }
    fn as_client(&self) -> Option<OpenedFileClient> {
        None
    }
}
#[derive(Debug, Clone)]
pub struct OpenedFileService {}
impl OpenedFileService {
    #[allow(dead_code)]
    pub(crate) fn attach(
        wasm_me: std::sync::Arc<dyn OpenedFile>,
        call_handle: wasmer_bus::abi::CallHandle,
    ) {
        {
            let wasm_me = wasm_me.clone();
            let call_handle = call_handle.clone();
            wasmer_bus::task::respond_to(
                call_handle,
                wasmer_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |wasm_handle: wasmer_bus::abi::CallHandle, wasm_req: OpenedFileMetaRequest| {
                    let wasm_me = wasm_me.clone();
                    async move {
                        match wasm_me.meta().await {
                            Ok(res) => wasmer_bus::abi::RespondActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::RespondActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
        {
            let wasm_me = wasm_me.clone();
            let call_handle = call_handle.clone();
            wasmer_bus::task::respond_to(
                call_handle,
                wasmer_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |wasm_handle: wasmer_bus::abi::CallHandle, wasm_req: OpenedFileUnlinkRequest| {
                    let wasm_me = wasm_me.clone();
                    async move {
                        match wasm_me.unlink().await {
                            Ok(res) => wasmer_bus::abi::RespondActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::RespondActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
        {
            let wasm_me = wasm_me.clone();
            let call_handle = call_handle.clone();
            wasmer_bus::task::respond_to(
                call_handle,
                wasmer_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |wasm_handle: wasmer_bus::abi::CallHandle, wasm_req: OpenedFileSetLenRequest| {
                    let wasm_me = wasm_me.clone();
                    let len = wasm_req.len;
                    async move {
                        match wasm_me.set_len(len).await {
                            Ok(res) => wasmer_bus::abi::RespondActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::RespondActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
        {
            let wasm_me = wasm_me.clone();
            let call_handle = call_handle.clone();
            wasmer_bus::task::respond_to(
                call_handle,
                wasmer_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |wasm_handle: wasmer_bus::abi::CallHandle, wasm_req: OpenedFileIoRequest| {
                    let wasm_me = wasm_me.clone();
                    async move {
                        match wasm_me.io().await {
                            Ok(svc) => {
                                FileIOService::attach(svc, wasm_handle);
                                wasmer_bus::abi::RespondActionTyped::<()>::Detach
                            },
                            Err(err) => wasmer_bus::abi::RespondActionTyped::<()>::Fault(err)
                        }
                    }
                },
            );
        }
    }
    pub fn listen(wasm_me: std::sync::Arc<dyn OpenedFile>) {
        {
            let wasm_me = wasm_me.clone();
            wasmer_bus::task::listen(
                wasmer_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasmer_bus::abi::CallHandle, wasm_req: OpenedFileMetaRequest| {
                    let wasm_me = wasm_me.clone();
                    async move {
                        match wasm_me.meta().await {
                            Ok(res) => wasmer_bus::abi::ListenActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::ListenActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasmer_bus::task::listen(
                wasmer_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasmer_bus::abi::CallHandle,
                      wasm_req: OpenedFileUnlinkRequest| {
                    let wasm_me = wasm_me.clone();
                    async move {
                        match wasm_me.unlink().await {
                            Ok(res) => wasmer_bus::abi::ListenActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::ListenActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasmer_bus::task::listen(
                wasmer_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasmer_bus::abi::CallHandle,
                      wasm_req: OpenedFileSetLenRequest| {
                    let wasm_me = wasm_me.clone();
                    let len = wasm_req.len;
                    async move {
                        match wasm_me.set_len(len).await {
                            Ok(res) => wasmer_bus::abi::ListenActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::ListenActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasmer_bus::task::listen(
                wasmer_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |wasm_handle: wasmer_bus::abi::CallHandle, wasm_req: OpenedFileIoRequest| {
                    let wasm_me = wasm_me.clone();
                    async move {
                        match wasm_me.io().await {
                            Ok(svc) => {
                                FileIOService::attach(svc, wasm_handle);
                                wasmer_bus::abi::ListenActionTyped::<()>::Detach
                            },
                            Err(err) => wasmer_bus::abi::ListenActionTyped::<()>::Fault(err)
                        }
                    }
                },
            );
        }
    }
    pub fn serve() {
        wasmer_bus::task::serve();
    }
}
#[derive(Debug, Clone)]
pub struct OpenedFileClient {
    ctx: wasmer_bus::abi::CallContext,
    task: Option<wasmer_bus::abi::Call>,
    join: Option<wasmer_bus::abi::CallJoin<()>>,
}
impl OpenedFileClient {
    pub fn new(wapm: &str) -> Self {
        Self {
            ctx: wasmer_bus::abi::CallContext::NewBusCall {
                wapm: wapm.to_string().into(),
                instance: None,
            },
            task: None,
            join: None,
        }
    }
    pub fn new_with_instance(wapm: &str, instance: &str, access_token: &str) -> Self {
        Self {
            ctx: wasmer_bus::abi::CallContext::NewBusCall {
                wapm: wapm.to_string().into(),
                instance: Some(wasmer_bus::abi::CallInstance::new(instance, access_token)),
            },
            task: None,
            join: None,
        }
    }
    pub fn attach(handle: wasmer_bus::abi::CallHandle) -> Self {
        let handle = wasmer_bus::abi::CallSmartHandle::new(handle);
        Self {
            ctx: wasmer_bus::abi::CallContext::OwnedSubCall { parent: handle },
            task: None,
            join: None,
        }
    }
    pub fn wait(self) -> Result<(), wasmer_bus::abi::BusError> {
        if let Some(join) = self.join {
            join.wait()?;
        }
        if let Some(task) = self.task {
            task.join()?.wait()?;
        }
        Ok(())
    }
    pub fn try_wait(&mut self) -> Result<Option<()>, wasmer_bus::abi::BusError> {
        if let Some(task) = self.task.take() {
            self.join.replace(task.join()?);
        }
        if let Some(join) = self.join.as_mut() {
            join.try_wait()
        } else {
            Ok(None)
        }
    }
    pub async fn meta(&self) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError> {
        let request = OpenedFileMetaRequest {};
        wasmer_bus::abi::call(
            self.ctx.clone(),
            wasmer_bus::abi::SerializationFormat::Json,
            request,
        )
        .invoke()
        .join()?
        .await
    }
    pub async fn unlink(&self) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        let request = OpenedFileUnlinkRequest {};
        wasmer_bus::abi::call(
            self.ctx.clone(),
            wasmer_bus::abi::SerializationFormat::Json,
            request,
        )
        .invoke()
        .join()?
        .await
    }
    pub async fn set_len(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        let request = OpenedFileSetLenRequest { len };
        wasmer_bus::abi::call(
            self.ctx.clone(),
            wasmer_bus::abi::SerializationFormat::Json,
            request,
        )
        .invoke()
        .join()?
        .await
    }
    pub async fn io(
        &self,
    ) -> std::result::Result<std::sync::Arc<dyn FileIO>, wasmer_bus::abi::BusError> {
        let request = OpenedFileIoRequest {};
        let handle = wasmer_bus::abi::call(
            self.ctx.clone(),
            wasmer_bus::abi::SerializationFormat::Json,
            request,
        )
        .detach()?;
        Ok(Arc::new(FileIOClient::attach(handle)))
    }
    pub fn blocking_meta(
        &self,
    ) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError> {
        wasmer_bus::task::block_on(self.meta())
    }
    pub fn blocking_unlink(&self) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        wasmer_bus::task::block_on(self.unlink())
    }
    pub fn blocking_set_len(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        wasmer_bus::task::block_on(self.set_len(len))
    }
    pub fn blocking_io(
        &self,
    ) -> std::result::Result<std::sync::Arc<dyn FileIO>, wasmer_bus::abi::BusError> {
        wasmer_bus::task::block_on(self.io())
    }
}
impl std::future::Future for OpenedFileClient {
    type Output = Result<(), wasmer_bus::abi::BusError>;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if let Some(task) = self.task.take() {
            self.join.replace(task.join()?);
        }
        if let Some(join) = self.join.as_mut() {
            let join = std::pin::Pin::new(join);
            return join.poll(cx);
        } else {
            std::task::Poll::Ready(Ok(()))
        }
    }
}
#[wasmer_bus::async_trait]
impl OpenedFile for OpenedFileClient {
    async fn meta(&self) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError> {
        OpenedFileClient::meta(self).await
    }
    fn blocking_meta(&self) -> std::result::Result<FsResult<Metadata>, wasmer_bus::abi::BusError> {
        OpenedFileClient::blocking_meta(self)
    }
    async fn unlink(&self) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        OpenedFileClient::unlink(self).await
    }
    fn blocking_unlink(&self) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        OpenedFileClient::blocking_unlink(self)
    }
    async fn set_len(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        OpenedFileClient::set_len(self, len).await
    }
    fn blocking_set_len(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        OpenedFileClient::blocking_set_len(self, len)
    }
    async fn io(&self) -> std::result::Result<std::sync::Arc<dyn FileIO>, wasmer_bus::abi::BusError> {
        OpenedFileClient::io(self).await
    }
    fn blocking_io(
        &self,
    ) -> std::result::Result<std::sync::Arc<dyn FileIO>, wasmer_bus::abi::BusError> {
        OpenedFileClient::blocking_io(self)
    }
    fn as_client(&self) -> Option<OpenedFileClient> {
        Some(self.clone())
    }
}

#[derive(Debug, Clone, serde :: Serialize, serde :: Deserialize)]
pub struct FileIoSeekRequest {
    pub from: SeekFrom,
}
#[derive(Debug, Clone, serde :: Serialize, serde :: Deserialize)]
pub struct FileIoFlushRequest {}
#[derive(Debug, Clone, serde :: Serialize, serde :: Deserialize)]
pub struct FileIoWriteRequest {
    pub data: Vec<u8>,
}
#[derive(Debug, Clone, serde :: Serialize, serde :: Deserialize)]
pub struct FileIoReadRequest {
    pub len: u64,
}
#[wasmer_bus::async_trait]
pub trait FileIO
where
    Self: std::fmt::Debug + Send + Sync,
{
    async fn seek(
        &self,
        from: SeekFrom,
    ) -> std::result::Result<FsResult<u64>, wasmer_bus::abi::BusError>;
    async fn flush(&self) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError>;
    async fn write(
        &self,
        data: Vec<u8>,
    ) -> std::result::Result<FsResult<u64>, wasmer_bus::abi::BusError>;
    async fn read(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<Vec<u8>>, wasmer_bus::abi::BusError>;
    fn blocking_seek(
        &self,
        from: SeekFrom,
    ) -> std::result::Result<FsResult<u64>, wasmer_bus::abi::BusError>;
    fn blocking_flush(&self) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError>;
    fn blocking_write(
        &self,
        data: Vec<u8>,
    ) -> std::result::Result<FsResult<u64>, wasmer_bus::abi::BusError>;
    fn blocking_read(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<Vec<u8>>, wasmer_bus::abi::BusError>;
    fn as_client(&self) -> Option<FileIOClient>;
}
#[wasmer_bus::async_trait]
pub trait FileIOSimplified
where
    Self: std::fmt::Debug + Send + Sync,
{
    async fn seek(&self, from: SeekFrom) -> FsResult<u64>;
    async fn flush(&self) -> FsResult<()>;
    async fn write(&self, data: Vec<u8>) -> FsResult<u64>;
    async fn read(&self, len: u64) -> FsResult<Vec<u8>>;
}
#[wasmer_bus::async_trait]
impl<T> FileIO for T
where
    T: FileIOSimplified,
{
    async fn seek(
        &self,
        from: SeekFrom,
    ) -> std::result::Result<FsResult<u64>, wasmer_bus::abi::BusError> {
        Ok(FileIOSimplified::seek(self, from).await)
    }
    fn blocking_seek(
        &self,
        from: SeekFrom,
    ) -> std::result::Result<FsResult<u64>, wasmer_bus::abi::BusError> {
        Ok(wasmer_bus::task::block_on(FileIOSimplified::seek(self, from)))
    }
    async fn flush(&self) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        Ok(FileIOSimplified::flush(self).await)
    }
    fn blocking_flush(&self) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        Ok(wasmer_bus::task::block_on(FileIOSimplified::flush(self)))
    }
    async fn write(
        &self,
        data: Vec<u8>,
    ) -> std::result::Result<FsResult<u64>, wasmer_bus::abi::BusError> {
        Ok(FileIOSimplified::write(self, data).await)
    }
    fn blocking_write(
        &self,
        data: Vec<u8>,
    ) -> std::result::Result<FsResult<u64>, wasmer_bus::abi::BusError> {
        Ok(wasmer_bus::task::block_on(FileIOSimplified::write(
            self, data,
        )))
    }
    async fn read(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<Vec<u8>>, wasmer_bus::abi::BusError> {
        Ok(FileIOSimplified::read(self, len).await)
    }
    fn blocking_read(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<Vec<u8>>, wasmer_bus::abi::BusError> {
        Ok(wasmer_bus::task::block_on(FileIOSimplified::read(self, len)))
    }
    fn as_client(&self) -> Option<FileIOClient> {
        None
    }
}
#[derive(Debug, Clone)]
pub struct FileIOService {}
impl FileIOService {
    #[allow(dead_code)]
    pub(crate) fn attach(
        wasm_me: std::sync::Arc<dyn FileIO>,
        call_handle: wasmer_bus::abi::CallHandle,
    ) {
        {
            let wasm_me = wasm_me.clone();
            let call_handle = call_handle.clone();
            wasmer_bus::task::respond_to(
                call_handle,
                wasmer_bus::abi::SerializationFormat::Bincode,
                #[allow(unused_variables)]
                move |wasm_handle: wasmer_bus::abi::CallHandle, wasm_req: FileIoSeekRequest| {
                    let wasm_me = wasm_me.clone();
                    let from = wasm_req.from;
                    async move {
                        match wasm_me.seek(from).await {
                            Ok(res) => wasmer_bus::abi::RespondActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::RespondActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
        {
            let wasm_me = wasm_me.clone();
            let call_handle = call_handle.clone();
            wasmer_bus::task::respond_to(
                call_handle,
                wasmer_bus::abi::SerializationFormat::Bincode,
                #[allow(unused_variables)]
                move |wasm_handle: wasmer_bus::abi::CallHandle, wasm_req: FileIoFlushRequest| {
                    let wasm_me = wasm_me.clone();
                    async move {
                        match wasm_me.flush().await {
                            Ok(res) => wasmer_bus::abi::RespondActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::RespondActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
        {
            let wasm_me = wasm_me.clone();
            let call_handle = call_handle.clone();
            wasmer_bus::task::respond_to(
                call_handle,
                wasmer_bus::abi::SerializationFormat::Bincode,
                #[allow(unused_variables)]
                move |wasm_handle: wasmer_bus::abi::CallHandle, wasm_req: FileIoWriteRequest| {
                    let wasm_me = wasm_me.clone();
                    let data = wasm_req.data;
                    async move {
                        match wasm_me.write(data).await {
                            Ok(res) => wasmer_bus::abi::RespondActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::RespondActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
        {
            let wasm_me = wasm_me.clone();
            let call_handle = call_handle.clone();
            wasmer_bus::task::respond_to(
                call_handle,
                wasmer_bus::abi::SerializationFormat::Bincode,
                #[allow(unused_variables)]
                move |wasm_handle: wasmer_bus::abi::CallHandle, wasm_req: FileIoReadRequest| {
                    let wasm_me = wasm_me.clone();
                    let len = wasm_req.len;
                    async move {
                        match wasm_me.read(len).await {
                            Ok(res) => wasmer_bus::abi::RespondActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::RespondActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
    }
    pub fn listen(wasm_me: std::sync::Arc<dyn FileIO>) {
        {
            let wasm_me = wasm_me.clone();
            wasmer_bus::task::listen(
                wasmer_bus::abi::SerializationFormat::Bincode,
                #[allow(unused_variables)]
                move |_wasm_handle: wasmer_bus::abi::CallHandle, wasm_req: FileIoSeekRequest| {
                    let wasm_me = wasm_me.clone();
                    let from = wasm_req.from;
                    async move {
                        match wasm_me.seek(from).await {
                            Ok(res) => wasmer_bus::abi::ListenActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::ListenActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasmer_bus::task::listen(
                wasmer_bus::abi::SerializationFormat::Bincode,
                #[allow(unused_variables)]
                move |_wasm_handle: wasmer_bus::abi::CallHandle, wasm_req: FileIoFlushRequest| {
                    let wasm_me = wasm_me.clone();
                    async move {
                        match wasm_me.flush().await {
                            Ok(res) => wasmer_bus::abi::ListenActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::ListenActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasmer_bus::task::listen(
                wasmer_bus::abi::SerializationFormat::Bincode,
                #[allow(unused_variables)]
                move |_wasm_handle: wasmer_bus::abi::CallHandle, wasm_req: FileIoWriteRequest| {
                    let wasm_me = wasm_me.clone();
                    let data = wasm_req.data;
                    async move {
                        match wasm_me.write(data).await {
                            Ok(res) => wasmer_bus::abi::ListenActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::ListenActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasmer_bus::task::listen(
                wasmer_bus::abi::SerializationFormat::Bincode,
                #[allow(unused_variables)]
                move |_wasm_handle: wasmer_bus::abi::CallHandle, wasm_req: FileIoReadRequest| {
                    let wasm_me = wasm_me.clone();
                    let len = wasm_req.len;
                    async move {
                        match wasm_me.read(len).await {
                            Ok(res) => wasmer_bus::abi::ListenActionTyped::Response(res),
                            Err(err) => wasmer_bus::abi::ListenActionTyped::Fault(err)
                        }
                    }
                },
            );
        }
    }
    pub fn serve() {
        wasmer_bus::task::serve();
    }
}
#[derive(Debug, Clone)]
pub struct FileIOClient {
    ctx: wasmer_bus::abi::CallContext,
    task: Option<wasmer_bus::abi::Call>,
    join: Option<wasmer_bus::abi::CallJoin<()>>,
}
impl FileIOClient {
    pub fn new(wapm: &str) -> Self {
        Self {
            ctx: wasmer_bus::abi::CallContext::NewBusCall {
                wapm: wapm.to_string().into(),
                instance: None,
            },
            task: None,
            join: None,
        }
    }
    pub fn new_with_instance(wapm: &str, instance: &str, access_token: &str) -> Self {
        Self {
            ctx: wasmer_bus::abi::CallContext::NewBusCall {
                wapm: wapm.to_string().into(),
                instance: Some(wasmer_bus::abi::CallInstance::new(instance, access_token)),
            },
            task: None,
            join: None,
        }
    }
    pub fn attach(handle: wasmer_bus::abi::CallHandle) -> Self {
        let handle = wasmer_bus::abi::CallSmartHandle::new(handle);
        Self {
            ctx: wasmer_bus::abi::CallContext::OwnedSubCall { parent: handle },
            task: None,
            join: None,
        }
    }
    pub fn wait(self) -> Result<(), wasmer_bus::abi::BusError> {
        if let Some(join) = self.join {
            join.wait()?;
        }
        if let Some(task) = self.task {
            task.join()?.wait()?;
        }
        Ok(())
    }
    pub fn try_wait(&mut self) -> Result<Option<()>, wasmer_bus::abi::BusError> {
        if let Some(task) = self.task.take() {
            self.join.replace(task.join()?);
        }
        if let Some(join) = self.join.as_mut() {
            join.try_wait()
        } else {
            Ok(None)
        }
    }
    pub async fn seek(
        &self,
        from: SeekFrom,
    ) -> std::result::Result<FsResult<u64>, wasmer_bus::abi::BusError> {
        let request = FileIoSeekRequest { from };
        wasmer_bus::abi::call(
            self.ctx.clone(),
            wasmer_bus::abi::SerializationFormat::Bincode,
            request,
        )
        .invoke()
        .join()?
        .await
    }
    pub async fn flush(&self) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        let request = FileIoFlushRequest {};
        wasmer_bus::abi::call(
            self.ctx.clone(),
            wasmer_bus::abi::SerializationFormat::Bincode,
            request,
        )
        .invoke()
        .join()?
        .await
    }
    pub async fn write(
        &self,
        data: Vec<u8>,
    ) -> std::result::Result<FsResult<u64>, wasmer_bus::abi::BusError> {
        let request = FileIoWriteRequest { data };
        wasmer_bus::abi::call(
            self.ctx.clone(),
            wasmer_bus::abi::SerializationFormat::Bincode,
            request,
        )
        .invoke()
        .join()?
        .await
    }
    pub async fn read(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<Vec<u8>>, wasmer_bus::abi::BusError> {
        let request = FileIoReadRequest { len };
        wasmer_bus::abi::call(
            self.ctx.clone(),
            wasmer_bus::abi::SerializationFormat::Bincode,
            request,
        )
        .invoke()
        .join()?
        .await
    }
    pub fn blocking_seek(
        &self,
        from: SeekFrom,
    ) -> std::result::Result<FsResult<u64>, wasmer_bus::abi::BusError> {
        wasmer_bus::task::block_on(self.seek(from))
    }
    pub fn blocking_flush(&self) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        wasmer_bus::task::block_on(self.flush())
    }
    pub fn blocking_write(
        &self,
        data: Vec<u8>,
    ) -> std::result::Result<FsResult<u64>, wasmer_bus::abi::BusError> {
        wasmer_bus::task::block_on(self.write(data))
    }
    pub fn blocking_read(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<Vec<u8>>, wasmer_bus::abi::BusError> {
        wasmer_bus::task::block_on(self.read(len))
    }
}
impl std::future::Future for FileIOClient {
    type Output = Result<(), wasmer_bus::abi::BusError>;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if let Some(task) = self.task.take() {
            self.join.replace(task.join()?);
        }
        if let Some(join) = self.join.as_mut() {
            let join = std::pin::Pin::new(join);
            return join.poll(cx);
        } else {
            std::task::Poll::Ready(Ok(()))
        }
    }
}
#[wasmer_bus::async_trait]
impl FileIO for FileIOClient {
    async fn seek(
        &self,
        from: SeekFrom,
    ) -> std::result::Result<FsResult<u64>, wasmer_bus::abi::BusError> {
        FileIOClient::seek(self, from).await
    }
    fn blocking_seek(
        &self,
        from: SeekFrom,
    ) -> std::result::Result<FsResult<u64>, wasmer_bus::abi::BusError> {
        FileIOClient::blocking_seek(self, from)
    }
    async fn flush(&self) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        FileIOClient::flush(self).await
    }
    fn blocking_flush(&self) -> std::result::Result<FsResult<()>, wasmer_bus::abi::BusError> {
        FileIOClient::blocking_flush(self)
    }
    async fn write(
        &self,
        data: Vec<u8>,
    ) -> std::result::Result<FsResult<u64>, wasmer_bus::abi::BusError> {
        FileIOClient::write(self, data).await
    }
    fn blocking_write(
        &self,
        data: Vec<u8>,
    ) -> std::result::Result<FsResult<u64>, wasmer_bus::abi::BusError> {
        FileIOClient::blocking_write(self, data)
    }
    async fn read(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<Vec<u8>>, wasmer_bus::abi::BusError> {
        FileIOClient::read(self, len).await
    }
    fn blocking_read(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<Vec<u8>>, wasmer_bus::abi::BusError> {
        FileIOClient::blocking_read(self, len)
    }
    fn as_client(&self) -> Option<FileIOClient> {
        Some(self.clone())
    }
}
*/
