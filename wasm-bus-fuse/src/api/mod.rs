use serde::*;
use std::io;
use std::sync::Arc;
#[allow(unused_imports)]
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
#[async_trait::async_trait]
pub trait Fuse
where
    Self: std::fmt::Debug + Send + Sync,
{
    async fn mount(
        &self,
        name: String,
    ) -> std::result::Result<
        std::sync::Arc<dyn FileSystem + Send + Sync + 'static>,
        wasm_bus::abi::CallError,
    >;
    fn blocking_mount(
        &self,
        name: String,
    ) -> std::result::Result<
        std::sync::Arc<dyn FileSystem + Send + Sync + 'static>,
        wasm_bus::abi::CallError,
    >;
    fn as_client(&self) -> Option<FuseClient>;
    fn handle(&self) -> Option<wasm_bus::abi::CallHandle>;
}
#[async_trait::async_trait]
pub trait FuseSimplified
where
    Self: std::fmt::Debug + Send + Sync,
{
    async fn mount(
        &self,
        name: String,
    ) -> std::result::Result<
        std::sync::Arc<dyn FileSystem + Send + Sync + 'static>,
        wasm_bus::abi::CallError,
    >;
}
#[async_trait::async_trait]
impl<T> Fuse for T
where
    T: FuseSimplified,
{
    async fn mount(
        &self,
        name: String,
    ) -> std::result::Result<
        std::sync::Arc<dyn FileSystem + Send + Sync + 'static>,
        wasm_bus::abi::CallError,
    > {
        FuseSimplified::mount(self, name).await
    }
    fn blocking_mount(
        &self,
        name: String,
    ) -> std::result::Result<
        std::sync::Arc<dyn FileSystem + Send + Sync + 'static>,
        wasm_bus::abi::CallError,
    > {
        wasm_bus::task::block_on(FuseSimplified::mount(self, name))
    }
    fn as_client(&self) -> Option<FuseClient> {
        None
    }
    fn handle(&self) -> Option<wasm_bus::abi::CallHandle> {
        None
    }
}
#[derive(Debug, Clone)]
pub struct FuseService {}
impl FuseService {
    #[allow(dead_code)]
    pub(crate) fn attach(
        wasm_me: std::sync::Arc<dyn Fuse + Send + Sync + 'static>,
        call_handle: wasm_bus::abi::CallHandle,
    ) {
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::respond_to(
                call_handle,
                wasm_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |wasm_handle: wasm_bus::abi::CallHandle, wasm_req: FuseMountRequest| {
                    let wasm_me = wasm_me.clone();
                    let name = wasm_req.name;
                    async move {
                        let svc = wasm_me.mount(name).await?;
                        FileSystemService::attach(svc, wasm_handle);
                        Ok(())
                    }
                },
                true,
            );
        }
    }
    pub fn listen(wasm_me: std::sync::Arc<dyn Fuse + Send + Sync + 'static>) {
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::listen(
                wasm_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |wasm_handle: wasm_bus::abi::CallHandle, wasm_req: FuseMountRequest| {
                    let wasm_me = wasm_me.clone();
                    let name = wasm_req.name;
                    async move {
                        let svc = wasm_me.mount(name).await?;
                        FileSystemService::attach(svc, wasm_handle);
                        Ok(())
                    }
                },
                true,
            );
        }
    }
    pub fn serve() {
        wasm_bus::task::serve();
    }
}
#[derive(Debug, Clone)]
pub struct FuseClient {
    wapm: std::borrow::Cow<'static, str>,
    instance: Option<wasm_bus::abi::CallInstance>,
    parent: Option<std::sync::Arc<wasm_bus::abi::DetachedCall<()>>>,
    task: Option<wasm_bus::abi::Call>,
    join: Option<wasm_bus::abi::CallJoin<()>>,
}
impl FuseClient {
    pub fn new(wapm: &str) -> Self {
        Self {
            wapm: wapm.to_string().into(),
            instance: None,
            parent: None,
            task: None,
            join: None,
        }
    }
    pub fn new_with_instance(wapm: &str, instance: &str, access_token: &str) -> Self {
        Self {
            wapm: wapm.to_string().into(),
            instance: Some(wasm_bus::abi::CallInstance::new(instance, access_token)),
            parent: None,
            task: None,
            join: None,
        }
    }
    pub fn attach(task: wasm_bus::abi::DetachedCall<()>) -> Self {
        let wapm = task.wapm();
        let instance = task.clone_instance();
        Self {
            wapm,
            instance,
            parent: Some(std::sync::Arc::new(task)),
            task: None,
            join: None,
        }
    }
    pub fn id(&self) -> u32 {
        self.task.as_ref().map(|a| a.id()).unwrap_or(0u32)
    }
    pub fn handle(&self) -> Option<wasm_bus::abi::CallHandle> {
        if let Some(handle) = self.task.as_ref().map(|a| a.handle()) {
            return Some(handle);
        }
        None
    }
    pub fn wait(self) -> Result<(), wasm_bus::abi::CallError> {
        if let Some(join) = self.join {
            join.wait()?;
        }
        if let Some(task) = self.task {
            task.join().wait()?;
        }
        Ok(())
    }
    pub fn try_wait(&mut self) -> Result<Option<()>, wasm_bus::abi::CallError> {
        if let Some(task) = self.task.take() {
            self.join.replace(task.join());
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
    ) -> std::result::Result<
        std::sync::Arc<dyn FileSystem + Send + Sync + 'static>,
        wasm_bus::abi::CallError,
    > {
        let request = FuseMountRequest { name };
        let task = wasm_bus::abi::call_ext(
            self.parent.as_ref().map(|a| a.handle()),
            self.wapm.clone(),
            wasm_bus::abi::SerializationFormat::Json,
            self.instance.clone(),
            request,
        )
        .detach()
        .await?;
        Ok(Arc::new(FileSystemClient::attach(task)))
    }
    pub fn blocking_mount(
        &self,
        name: String,
    ) -> std::result::Result<
        std::sync::Arc<dyn FileSystem + Send + Sync + 'static>,
        wasm_bus::abi::CallError,
    > {
        wasm_bus::task::block_on(self.mount(name))
    }
}
impl std::future::Future for FuseClient {
    type Output = Result<(), wasm_bus::abi::CallError>;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if let Some(task) = self.task.take() {
            self.join.replace(task.join());
        }
        if let Some(join) = self.join.as_mut() {
            let join = std::pin::Pin::new(join);
            return join.poll(cx);
        } else {
            std::task::Poll::Ready(Ok(()))
        }
    }
}
#[async_trait::async_trait]
impl Fuse for FuseClient {
    async fn mount(
        &self,
        name: String,
    ) -> std::result::Result<
        std::sync::Arc<dyn FileSystem + Send + Sync + 'static>,
        wasm_bus::abi::CallError,
    > {
        FuseClient::mount(self, name).await
    }
    fn blocking_mount(
        &self,
        name: String,
    ) -> std::result::Result<
        std::sync::Arc<dyn FileSystem + Send + Sync + 'static>,
        wasm_bus::abi::CallError,
    > {
        FuseClient::blocking_mount(self, name)
    }
    fn as_client(&self) -> Option<FuseClient> {
        Some(self.clone())
    }
    fn handle(&self) -> Option<wasm_bus::abi::CallHandle> {
        FuseClient::handle(self)
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
#[async_trait::async_trait]
pub trait FileSystem
where
    Self: std::fmt::Debug + Send + Sync,
{
    async fn init(&self) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError>;
    async fn read_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Dir>, wasm_bus::abi::CallError>;
    async fn create_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError>;
    async fn remove_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError>;
    async fn rename(
        &self,
        from: String,
        to: String,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError>;
    async fn remove_file(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError>;
    async fn read_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError>;
    async fn read_symlink_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError>;
    async fn open(
        &self,
        path: String,
        options: OpenOptions,
    ) -> std::result::Result<
        std::sync::Arc<dyn OpenedFile + Send + Sync + 'static>,
        wasm_bus::abi::CallError,
    >;
    fn blocking_init(&self) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError>;
    fn blocking_read_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Dir>, wasm_bus::abi::CallError>;
    fn blocking_create_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError>;
    fn blocking_remove_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError>;
    fn blocking_rename(
        &self,
        from: String,
        to: String,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError>;
    fn blocking_remove_file(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError>;
    fn blocking_read_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError>;
    fn blocking_read_symlink_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError>;
    fn blocking_open(
        &self,
        path: String,
        options: OpenOptions,
    ) -> std::result::Result<
        std::sync::Arc<dyn OpenedFile + Send + Sync + 'static>,
        wasm_bus::abi::CallError,
    >;
    fn as_client(&self) -> Option<FileSystemClient>;
    fn handle(&self) -> Option<wasm_bus::abi::CallHandle>;
    fn parent_handle(&self) -> Option<wasm_bus::abi::CallHandle>;
}
#[async_trait::async_trait]
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
    ) -> std::result::Result<
        std::sync::Arc<dyn OpenedFile + Send + Sync + 'static>,
        wasm_bus::abi::CallError,
    >;
}
#[async_trait::async_trait]
impl<T> FileSystem for T
where
    T: FileSystemSimplified,
{
    async fn init(&self) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        Ok(FileSystemSimplified::init(self).await)
    }
    fn blocking_init(&self) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        Ok(wasm_bus::task::block_on(FileSystemSimplified::init(self)))
    }
    async fn read_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Dir>, wasm_bus::abi::CallError> {
        Ok(FileSystemSimplified::read_dir(self, path).await)
    }
    fn blocking_read_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Dir>, wasm_bus::abi::CallError> {
        Ok(wasm_bus::task::block_on(FileSystemSimplified::read_dir(
            self, path,
        )))
    }
    async fn create_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError> {
        Ok(FileSystemSimplified::create_dir(self, path).await)
    }
    fn blocking_create_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError> {
        Ok(wasm_bus::task::block_on(FileSystemSimplified::create_dir(
            self, path,
        )))
    }
    async fn remove_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        Ok(FileSystemSimplified::remove_dir(self, path).await)
    }
    fn blocking_remove_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        Ok(wasm_bus::task::block_on(FileSystemSimplified::remove_dir(
            self, path,
        )))
    }
    async fn rename(
        &self,
        from: String,
        to: String,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        Ok(FileSystemSimplified::rename(self, from, to).await)
    }
    fn blocking_rename(
        &self,
        from: String,
        to: String,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        Ok(wasm_bus::task::block_on(FileSystemSimplified::rename(
            self, from, to,
        )))
    }
    async fn remove_file(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        Ok(FileSystemSimplified::remove_file(self, path).await)
    }
    fn blocking_remove_file(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        Ok(wasm_bus::task::block_on(FileSystemSimplified::remove_file(
            self, path,
        )))
    }
    async fn read_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError> {
        Ok(FileSystemSimplified::read_metadata(self, path).await)
    }
    fn blocking_read_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError> {
        Ok(wasm_bus::task::block_on(
            FileSystemSimplified::read_metadata(self, path),
        ))
    }
    async fn read_symlink_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError> {
        Ok(FileSystemSimplified::read_symlink_metadata(self, path).await)
    }
    fn blocking_read_symlink_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError> {
        Ok(wasm_bus::task::block_on(
            FileSystemSimplified::read_symlink_metadata(self, path),
        ))
    }
    async fn open(
        &self,
        path: String,
        options: OpenOptions,
    ) -> std::result::Result<
        std::sync::Arc<dyn OpenedFile + Send + Sync + 'static>,
        wasm_bus::abi::CallError,
    > {
        FileSystemSimplified::open(self, path, options).await
    }
    fn blocking_open(
        &self,
        path: String,
        options: OpenOptions,
    ) -> std::result::Result<
        std::sync::Arc<dyn OpenedFile + Send + Sync + 'static>,
        wasm_bus::abi::CallError,
    > {
        wasm_bus::task::block_on(FileSystemSimplified::open(self, path, options))
    }
    fn as_client(&self) -> Option<FileSystemClient> {
        None
    }
    fn handle(&self) -> Option<wasm_bus::abi::CallHandle> {
        None
    }
    fn parent_handle(&self) -> Option<wasm_bus::abi::CallHandle> {
        None
    }
}
#[derive(Debug, Clone)]
pub struct FileSystemService {}
impl FileSystemService {
    #[allow(dead_code)]
    pub(crate) fn attach(
        wasm_me: std::sync::Arc<dyn FileSystem + Send + Sync + 'static>,
        call_handle: wasm_bus::abi::CallHandle,
    ) {
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::respond_to(
                call_handle,
                wasm_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle, wasm_req: FileSystemInitRequest| {
                    let wasm_me = wasm_me.clone();
                    async move { wasm_me.init().await }
                },
                false,
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::respond_to(
                call_handle,
                wasm_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle,
                      wasm_req: FileSystemReadDirRequest| {
                    let wasm_me = wasm_me.clone();
                    let path = wasm_req.path;
                    async move { wasm_me.read_dir(path).await }
                },
                false,
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::respond_to(
                call_handle,
                wasm_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle,
                      wasm_req: FileSystemCreateDirRequest| {
                    let wasm_me = wasm_me.clone();
                    let path = wasm_req.path;
                    async move { wasm_me.create_dir(path).await }
                },
                false,
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::respond_to(
                call_handle,
                wasm_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle,
                      wasm_req: FileSystemRemoveDirRequest| {
                    let wasm_me = wasm_me.clone();
                    let path = wasm_req.path;
                    async move { wasm_me.remove_dir(path).await }
                },
                false,
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::respond_to(
                call_handle,
                wasm_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle,
                      wasm_req: FileSystemRenameRequest| {
                    let wasm_me = wasm_me.clone();
                    let from = wasm_req.from;
                    let to = wasm_req.to;
                    async move { wasm_me.rename(from, to).await }
                },
                false,
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::respond_to(
                call_handle,
                wasm_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle,
                      wasm_req: FileSystemRemoveFileRequest| {
                    let wasm_me = wasm_me.clone();
                    let path = wasm_req.path;
                    async move { wasm_me.remove_file(path).await }
                },
                false,
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::respond_to(
                call_handle,
                wasm_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle,
                      wasm_req: FileSystemReadMetadataRequest| {
                    let wasm_me = wasm_me.clone();
                    let path = wasm_req.path;
                    async move { wasm_me.read_metadata(path).await }
                },
                false,
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::respond_to(
                call_handle,
                wasm_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle,
                      wasm_req: FileSystemReadSymlinkMetadataRequest| {
                    let wasm_me = wasm_me.clone();
                    let path = wasm_req.path;
                    async move { wasm_me.read_symlink_metadata(path).await }
                },
                false,
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::respond_to(
                call_handle,
                wasm_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |wasm_handle: wasm_bus::abi::CallHandle, wasm_req: FileSystemOpenRequest| {
                    let wasm_me = wasm_me.clone();
                    let path = wasm_req.path;
                    let options = wasm_req.options;
                    async move {
                        let svc = wasm_me.open(path, options).await?;
                        OpenedFileService::attach(svc, wasm_handle);
                        Ok(())
                    }
                },
                true,
            );
        }
    }
    pub fn listen(wasm_me: std::sync::Arc<dyn FileSystem + Send + Sync + 'static>) {
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::listen(
                wasm_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle, wasm_req: FileSystemInitRequest| {
                    let wasm_me = wasm_me.clone();
                    async move { wasm_me.init().await }
                },
                false,
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::listen(
                wasm_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle,
                      wasm_req: FileSystemReadDirRequest| {
                    let wasm_me = wasm_me.clone();
                    let path = wasm_req.path;
                    async move { wasm_me.read_dir(path).await }
                },
                false,
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::listen(
                wasm_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle,
                      wasm_req: FileSystemCreateDirRequest| {
                    let wasm_me = wasm_me.clone();
                    let path = wasm_req.path;
                    async move { wasm_me.create_dir(path).await }
                },
                false,
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::listen(
                wasm_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle,
                      wasm_req: FileSystemRemoveDirRequest| {
                    let wasm_me = wasm_me.clone();
                    let path = wasm_req.path;
                    async move { wasm_me.remove_dir(path).await }
                },
                false,
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::listen(
                wasm_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle,
                      wasm_req: FileSystemRenameRequest| {
                    let wasm_me = wasm_me.clone();
                    let from = wasm_req.from;
                    let to = wasm_req.to;
                    async move { wasm_me.rename(from, to).await }
                },
                false,
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::listen(
                wasm_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle,
                      wasm_req: FileSystemRemoveFileRequest| {
                    let wasm_me = wasm_me.clone();
                    let path = wasm_req.path;
                    async move { wasm_me.remove_file(path).await }
                },
                false,
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::listen(
                wasm_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle,
                      wasm_req: FileSystemReadMetadataRequest| {
                    let wasm_me = wasm_me.clone();
                    let path = wasm_req.path;
                    async move { wasm_me.read_metadata(path).await }
                },
                false,
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::listen(
                wasm_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle,
                      wasm_req: FileSystemReadSymlinkMetadataRequest| {
                    let wasm_me = wasm_me.clone();
                    let path = wasm_req.path;
                    async move { wasm_me.read_symlink_metadata(path).await }
                },
                false,
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::listen(
                wasm_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |wasm_handle: wasm_bus::abi::CallHandle, wasm_req: FileSystemOpenRequest| {
                    let wasm_me = wasm_me.clone();
                    let path = wasm_req.path;
                    let options = wasm_req.options;
                    async move {
                        let svc = wasm_me.open(path, options).await?;
                        OpenedFileService::attach(svc, wasm_handle);
                        Ok(())
                    }
                },
                true,
            );
        }
    }
    pub fn serve() {
        wasm_bus::task::serve();
    }
}
#[derive(Debug, Clone)]
pub struct FileSystemClient {
    wapm: std::borrow::Cow<'static, str>,
    instance: Option<wasm_bus::abi::CallInstance>,
    parent: Option<std::sync::Arc<wasm_bus::abi::DetachedCall<()>>>,
    task: Option<wasm_bus::abi::Call>,
    join: Option<wasm_bus::abi::CallJoin<()>>,
}
impl FileSystemClient {
    pub fn new(wapm: &str) -> Self {
        Self {
            wapm: wapm.to_string().into(),
            instance: None,
            parent: None,
            task: None,
            join: None,
        }
    }
    pub fn new_with_instance(wapm: &str, instance: &str, access_token: &str) -> Self {
        Self {
            wapm: wapm.to_string().into(),
            instance: Some(wasm_bus::abi::CallInstance::new(instance, access_token)),
            parent: None,
            task: None,
            join: None,
        }
    }
    pub fn attach(task: wasm_bus::abi::DetachedCall<()>) -> Self {
        let wapm = task.wapm();
        let instance = task.clone_instance();
        Self {
            wapm,
            instance,
            parent: Some(std::sync::Arc::new(task)),
            task: None,
            join: None,
        }
    }
    pub fn id(&self) -> u32 {
        self.task.as_ref().map(|a| a.id()).unwrap_or(0u32)
    }
    pub fn handle(&self) -> Option<wasm_bus::abi::CallHandle> {
        if let Some(handle) = self.task.as_ref().map(|a| a.handle()) {
            return Some(handle);
        }
        None
    }
    pub fn parent_handle(&self) -> Option<wasm_bus::abi::CallHandle> {
        self.parent.as_ref().map(|a| a.handle())
    }
    pub fn wait(self) -> Result<(), wasm_bus::abi::CallError> {
        if let Some(join) = self.join {
            join.wait()?;
        }
        if let Some(task) = self.task {
            task.join().wait()?;
        }
        Ok(())
    }
    pub fn try_wait(&mut self) -> Result<Option<()>, wasm_bus::abi::CallError> {
        if let Some(task) = self.task.take() {
            self.join.replace(task.join());
        }
        if let Some(join) = self.join.as_mut() {
            join.try_wait()
        } else {
            Ok(None)
        }
    }
    pub async fn init(&self) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        let request = FileSystemInitRequest {};
        wasm_bus::abi::call_ext(
            self.parent.as_ref().map(|a| a.handle()),
            self.wapm.clone(),
            wasm_bus::abi::SerializationFormat::Json,
            self.instance.clone(),
            request,
        )
        .invoke()
        .join()
        .await
    }
    pub async fn read_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Dir>, wasm_bus::abi::CallError> {
        let request = FileSystemReadDirRequest { path };
        wasm_bus::abi::call_ext(
            self.parent.as_ref().map(|a| a.handle()),
            self.wapm.clone(),
            wasm_bus::abi::SerializationFormat::Json,
            self.instance.clone(),
            request,
        )
        .invoke()
        .join()
        .await
    }
    pub async fn create_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError> {
        let request = FileSystemCreateDirRequest { path };
        wasm_bus::abi::call_ext(
            self.parent.as_ref().map(|a| a.handle()),
            self.wapm.clone(),
            wasm_bus::abi::SerializationFormat::Json,
            self.instance.clone(),
            request,
        )
        .invoke()
        .join()
        .await
    }
    pub async fn remove_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        let request = FileSystemRemoveDirRequest { path };
        wasm_bus::abi::call_ext(
            self.parent.as_ref().map(|a| a.handle()),
            self.wapm.clone(),
            wasm_bus::abi::SerializationFormat::Json,
            self.instance.clone(),
            request,
        )
        .invoke()
        .join()
        .await
    }
    pub async fn rename(
        &self,
        from: String,
        to: String,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        let request = FileSystemRenameRequest { from, to };
        wasm_bus::abi::call_ext(
            self.parent.as_ref().map(|a| a.handle()),
            self.wapm.clone(),
            wasm_bus::abi::SerializationFormat::Json,
            self.instance.clone(),
            request,
        )
        .invoke()
        .join()
        .await
    }
    pub async fn remove_file(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        let request = FileSystemRemoveFileRequest { path };
        wasm_bus::abi::call_ext(
            self.parent.as_ref().map(|a| a.handle()),
            self.wapm.clone(),
            wasm_bus::abi::SerializationFormat::Json,
            self.instance.clone(),
            request,
        )
        .invoke()
        .join()
        .await
    }
    pub async fn read_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError> {
        let request = FileSystemReadMetadataRequest { path };
        wasm_bus::abi::call_ext(
            self.parent.as_ref().map(|a| a.handle()),
            self.wapm.clone(),
            wasm_bus::abi::SerializationFormat::Json,
            self.instance.clone(),
            request,
        )
        .invoke()
        .join()
        .await
    }
    pub async fn read_symlink_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError> {
        let request = FileSystemReadSymlinkMetadataRequest { path };
        wasm_bus::abi::call_ext(
            self.parent.as_ref().map(|a| a.handle()),
            self.wapm.clone(),
            wasm_bus::abi::SerializationFormat::Json,
            self.instance.clone(),
            request,
        )
        .invoke()
        .join()
        .await
    }
    pub async fn open(
        &self,
        path: String,
        options: OpenOptions,
    ) -> std::result::Result<
        std::sync::Arc<dyn OpenedFile + Send + Sync + 'static>,
        wasm_bus::abi::CallError,
    > {
        let request = FileSystemOpenRequest { path, options };
        let task = wasm_bus::abi::call_ext(
            self.parent.as_ref().map(|a| a.handle()),
            self.wapm.clone(),
            wasm_bus::abi::SerializationFormat::Json,
            self.instance.clone(),
            request,
        )
        .detach()
        .await?;
        Ok(Arc::new(OpenedFileClient::attach(task)))
    }
    pub fn blocking_init(&self) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        wasm_bus::task::block_on(self.init())
    }
    pub fn blocking_read_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Dir>, wasm_bus::abi::CallError> {
        wasm_bus::task::block_on(self.read_dir(path))
    }
    pub fn blocking_create_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError> {
        wasm_bus::task::block_on(self.create_dir(path))
    }
    pub fn blocking_remove_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        wasm_bus::task::block_on(self.remove_dir(path))
    }
    pub fn blocking_rename(
        &self,
        from: String,
        to: String,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        wasm_bus::task::block_on(self.rename(from, to))
    }
    pub fn blocking_remove_file(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        wasm_bus::task::block_on(self.remove_file(path))
    }
    pub fn blocking_read_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError> {
        wasm_bus::task::block_on(self.read_metadata(path))
    }
    pub fn blocking_read_symlink_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError> {
        wasm_bus::task::block_on(self.read_symlink_metadata(path))
    }
    pub fn blocking_open(
        &self,
        path: String,
        options: OpenOptions,
    ) -> std::result::Result<
        std::sync::Arc<dyn OpenedFile + Send + Sync + 'static>,
        wasm_bus::abi::CallError,
    > {
        wasm_bus::task::block_on(self.open(path, options))
    }
}
impl std::future::Future for FileSystemClient {
    type Output = Result<(), wasm_bus::abi::CallError>;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if let Some(task) = self.task.take() {
            self.join.replace(task.join());
        }
        if let Some(join) = self.join.as_mut() {
            let join = std::pin::Pin::new(join);
            return join.poll(cx);
        } else {
            std::task::Poll::Ready(Ok(()))
        }
    }
}
#[async_trait::async_trait]
impl FileSystem for FileSystemClient {
    async fn init(&self) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        FileSystemClient::init(self).await
    }
    fn blocking_init(&self) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        FileSystemClient::blocking_init(self)
    }
    async fn read_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Dir>, wasm_bus::abi::CallError> {
        FileSystemClient::read_dir(self, path).await
    }
    fn blocking_read_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Dir>, wasm_bus::abi::CallError> {
        FileSystemClient::blocking_read_dir(self, path)
    }
    async fn create_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError> {
        FileSystemClient::create_dir(self, path).await
    }
    fn blocking_create_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError> {
        FileSystemClient::blocking_create_dir(self, path)
    }
    async fn remove_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        FileSystemClient::remove_dir(self, path).await
    }
    fn blocking_remove_dir(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        FileSystemClient::blocking_remove_dir(self, path)
    }
    async fn rename(
        &self,
        from: String,
        to: String,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        FileSystemClient::rename(self, from, to).await
    }
    fn blocking_rename(
        &self,
        from: String,
        to: String,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        FileSystemClient::blocking_rename(self, from, to)
    }
    async fn remove_file(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        FileSystemClient::remove_file(self, path).await
    }
    fn blocking_remove_file(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        FileSystemClient::blocking_remove_file(self, path)
    }
    async fn read_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError> {
        FileSystemClient::read_metadata(self, path).await
    }
    fn blocking_read_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError> {
        FileSystemClient::blocking_read_metadata(self, path)
    }
    async fn read_symlink_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError> {
        FileSystemClient::read_symlink_metadata(self, path).await
    }
    fn blocking_read_symlink_metadata(
        &self,
        path: String,
    ) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError> {
        FileSystemClient::blocking_read_symlink_metadata(self, path)
    }
    async fn open(
        &self,
        path: String,
        options: OpenOptions,
    ) -> std::result::Result<
        std::sync::Arc<dyn OpenedFile + Send + Sync + 'static>,
        wasm_bus::abi::CallError,
    > {
        FileSystemClient::open(self, path, options).await
    }
    fn blocking_open(
        &self,
        path: String,
        options: OpenOptions,
    ) -> std::result::Result<
        std::sync::Arc<dyn OpenedFile + Send + Sync + 'static>,
        wasm_bus::abi::CallError,
    > {
        FileSystemClient::blocking_open(self, path, options)
    }
    fn as_client(&self) -> Option<FileSystemClient> {
        Some(self.clone())
    }
    fn handle(&self) -> Option<wasm_bus::abi::CallHandle> {
        FileSystemClient::handle(self)
    }
    fn parent_handle(&self) -> Option<wasm_bus::abi::CallHandle> {
        FileSystemClient::parent_handle(self)
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
#[async_trait::async_trait]
pub trait OpenedFile
where
    Self: std::fmt::Debug + Send + Sync,
{
    async fn meta(&self) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError>;
    async fn unlink(&self) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError>;
    async fn set_len(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError>;
    async fn io(
        &self,
    ) -> std::result::Result<
        std::sync::Arc<dyn FileIO + Send + Sync + 'static>,
        wasm_bus::abi::CallError,
    >;
    fn blocking_meta(&self) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError>;
    fn blocking_unlink(&self) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError>;
    fn blocking_set_len(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError>;
    fn blocking_io(
        &self,
    ) -> std::result::Result<
        std::sync::Arc<dyn FileIO + Send + Sync + 'static>,
        wasm_bus::abi::CallError,
    >;
    fn as_client(&self) -> Option<OpenedFileClient>;
    fn handle(&self) -> Option<wasm_bus::abi::CallHandle>;
}
#[async_trait::async_trait]
pub trait OpenedFileSimplified
where
    Self: std::fmt::Debug + Send + Sync,
{
    async fn meta(&self) -> FsResult<Metadata>;
    async fn unlink(&self) -> FsResult<()>;
    async fn set_len(&self, len: u64) -> FsResult<()>;
    async fn io(
        &self,
    ) -> std::result::Result<
        std::sync::Arc<dyn FileIO + Send + Sync + 'static>,
        wasm_bus::abi::CallError,
    >;
}
#[async_trait::async_trait]
impl<T> OpenedFile for T
where
    T: OpenedFileSimplified,
{
    async fn meta(&self) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError> {
        Ok(OpenedFileSimplified::meta(self).await)
    }
    fn blocking_meta(&self) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError> {
        Ok(wasm_bus::task::block_on(OpenedFileSimplified::meta(self)))
    }
    async fn unlink(&self) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        Ok(OpenedFileSimplified::unlink(self).await)
    }
    fn blocking_unlink(&self) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        Ok(wasm_bus::task::block_on(OpenedFileSimplified::unlink(self)))
    }
    async fn set_len(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        Ok(OpenedFileSimplified::set_len(self, len).await)
    }
    fn blocking_set_len(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        Ok(wasm_bus::task::block_on(OpenedFileSimplified::set_len(
            self, len,
        )))
    }
    async fn io(
        &self,
    ) -> std::result::Result<
        std::sync::Arc<dyn FileIO + Send + Sync + 'static>,
        wasm_bus::abi::CallError,
    > {
        OpenedFileSimplified::io(self).await
    }
    fn blocking_io(
        &self,
    ) -> std::result::Result<
        std::sync::Arc<dyn FileIO + Send + Sync + 'static>,
        wasm_bus::abi::CallError,
    > {
        wasm_bus::task::block_on(OpenedFileSimplified::io(self))
    }
    fn as_client(&self) -> Option<OpenedFileClient> {
        None
    }
    fn handle(&self) -> Option<wasm_bus::abi::CallHandle> {
        None
    }
}
#[derive(Debug, Clone)]
pub struct OpenedFileService {}
impl OpenedFileService {
    #[allow(dead_code)]
    pub(crate) fn attach(
        wasm_me: std::sync::Arc<dyn OpenedFile + Send + Sync + 'static>,
        call_handle: wasm_bus::abi::CallHandle,
    ) {
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::respond_to(
                call_handle,
                wasm_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle, wasm_req: OpenedFileMetaRequest| {
                    let wasm_me = wasm_me.clone();
                    async move { wasm_me.meta().await }
                },
                false,
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::respond_to(
                call_handle,
                wasm_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle,
                      wasm_req: OpenedFileUnlinkRequest| {
                    let wasm_me = wasm_me.clone();
                    async move { wasm_me.unlink().await }
                },
                false,
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::respond_to(
                call_handle,
                wasm_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle,
                      wasm_req: OpenedFileSetLenRequest| {
                    let wasm_me = wasm_me.clone();
                    let len = wasm_req.len;
                    async move { wasm_me.set_len(len).await }
                },
                false,
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::respond_to(
                call_handle,
                wasm_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |wasm_handle: wasm_bus::abi::CallHandle, wasm_req: OpenedFileIoRequest| {
                    let wasm_me = wasm_me.clone();
                    async move {
                        let svc = wasm_me.io().await?;
                        FileIOService::attach(svc, wasm_handle);
                        Ok(())
                    }
                },
                true,
            );
        }
    }
    pub fn listen(wasm_me: std::sync::Arc<dyn OpenedFile + Send + Sync + 'static>) {
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::listen(
                wasm_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle, wasm_req: OpenedFileMetaRequest| {
                    let wasm_me = wasm_me.clone();
                    async move { wasm_me.meta().await }
                },
                false,
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::listen(
                wasm_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle,
                      wasm_req: OpenedFileUnlinkRequest| {
                    let wasm_me = wasm_me.clone();
                    async move { wasm_me.unlink().await }
                },
                false,
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::listen(
                wasm_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle,
                      wasm_req: OpenedFileSetLenRequest| {
                    let wasm_me = wasm_me.clone();
                    let len = wasm_req.len;
                    async move { wasm_me.set_len(len).await }
                },
                false,
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::listen(
                wasm_bus::abi::SerializationFormat::Json,
                #[allow(unused_variables)]
                move |wasm_handle: wasm_bus::abi::CallHandle, wasm_req: OpenedFileIoRequest| {
                    let wasm_me = wasm_me.clone();
                    async move {
                        let svc = wasm_me.io().await?;
                        FileIOService::attach(svc, wasm_handle);
                        Ok(())
                    }
                },
                true,
            );
        }
    }
    pub fn serve() {
        wasm_bus::task::serve();
    }
}
#[derive(Debug, Clone)]
pub struct OpenedFileClient {
    wapm: std::borrow::Cow<'static, str>,
    instance: Option<wasm_bus::abi::CallInstance>,
    parent: Option<std::sync::Arc<wasm_bus::abi::DetachedCall<()>>>,
    task: Option<wasm_bus::abi::Call>,
    join: Option<wasm_bus::abi::CallJoin<()>>,
}
impl OpenedFileClient {
    pub fn new(wapm: &str) -> Self {
        Self {
            wapm: wapm.to_string().into(),
            instance: None,
            parent: None,
            task: None,
            join: None,
        }
    }
    pub fn new_with_instance(wapm: &str, instance: &str, access_token: &str) -> Self {
        Self {
            wapm: wapm.to_string().into(),
            instance: Some(wasm_bus::abi::CallInstance::new(instance, access_token)),
            parent: None,
            task: None,
            join: None,
        }
    }
    pub fn attach(task: wasm_bus::abi::DetachedCall<()>) -> Self {
        let wapm = task.wapm();
        let instance = task.clone_instance();
        Self {
            wapm,
            instance,
            parent: Some(std::sync::Arc::new(task)),
            task: None,
            join: None,
        }
    }
    pub fn id(&self) -> u32 {
        self.task.as_ref().map(|a| a.id()).unwrap_or(0u32)
    }
    pub fn handle(&self) -> Option<wasm_bus::abi::CallHandle> {
        if let Some(handle) = self.task.as_ref().map(|a| a.handle()) {
            return Some(handle);
        }
        None
    }
    pub fn wait(self) -> Result<(), wasm_bus::abi::CallError> {
        if let Some(join) = self.join {
            join.wait()?;
        }
        if let Some(task) = self.task {
            task.join().wait()?;
        }
        Ok(())
    }
    pub fn try_wait(&mut self) -> Result<Option<()>, wasm_bus::abi::CallError> {
        if let Some(task) = self.task.take() {
            self.join.replace(task.join());
        }
        if let Some(join) = self.join.as_mut() {
            join.try_wait()
        } else {
            Ok(None)
        }
    }
    pub async fn meta(&self) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError> {
        let request = OpenedFileMetaRequest {};
        wasm_bus::abi::call_ext(
            self.parent.as_ref().map(|a| a.handle()),
            self.wapm.clone(),
            wasm_bus::abi::SerializationFormat::Json,
            self.instance.clone(),
            request,
        )
        .invoke()
        .join()
        .await
    }
    pub async fn unlink(&self) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        let request = OpenedFileUnlinkRequest {};
        wasm_bus::abi::call_ext(
            self.parent.as_ref().map(|a| a.handle()),
            self.wapm.clone(),
            wasm_bus::abi::SerializationFormat::Json,
            self.instance.clone(),
            request,
        )
        .invoke()
        .join()
        .await
    }
    pub async fn set_len(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        let request = OpenedFileSetLenRequest { len };
        wasm_bus::abi::call_ext(
            self.parent.as_ref().map(|a| a.handle()),
            self.wapm.clone(),
            wasm_bus::abi::SerializationFormat::Json,
            self.instance.clone(),
            request,
        )
        .invoke()
        .join()
        .await
    }
    pub async fn io(
        &self,
    ) -> std::result::Result<
        std::sync::Arc<dyn FileIO + Send + Sync + 'static>,
        wasm_bus::abi::CallError,
    > {
        let request = OpenedFileIoRequest {};
        let task = wasm_bus::abi::call_ext(
            self.parent.as_ref().map(|a| a.handle()),
            self.wapm.clone(),
            wasm_bus::abi::SerializationFormat::Json,
            self.instance.clone(),
            request,
        )
        .detach()
        .await?;
        Ok(Arc::new(FileIOClient::attach(task)))
    }
    pub fn blocking_meta(
        &self,
    ) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError> {
        wasm_bus::task::block_on(self.meta())
    }
    pub fn blocking_unlink(&self) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        wasm_bus::task::block_on(self.unlink())
    }
    pub fn blocking_set_len(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        wasm_bus::task::block_on(self.set_len(len))
    }
    pub fn blocking_io(
        &self,
    ) -> std::result::Result<
        std::sync::Arc<dyn FileIO + Send + Sync + 'static>,
        wasm_bus::abi::CallError,
    > {
        wasm_bus::task::block_on(self.io())
    }
}
impl std::future::Future for OpenedFileClient {
    type Output = Result<(), wasm_bus::abi::CallError>;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if let Some(task) = self.task.take() {
            self.join.replace(task.join());
        }
        if let Some(join) = self.join.as_mut() {
            let join = std::pin::Pin::new(join);
            return join.poll(cx);
        } else {
            std::task::Poll::Ready(Ok(()))
        }
    }
}
#[async_trait::async_trait]
impl OpenedFile for OpenedFileClient {
    async fn meta(&self) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError> {
        OpenedFileClient::meta(self).await
    }
    fn blocking_meta(&self) -> std::result::Result<FsResult<Metadata>, wasm_bus::abi::CallError> {
        OpenedFileClient::blocking_meta(self)
    }
    async fn unlink(&self) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        OpenedFileClient::unlink(self).await
    }
    fn blocking_unlink(&self) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        OpenedFileClient::blocking_unlink(self)
    }
    async fn set_len(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        OpenedFileClient::set_len(self, len).await
    }
    fn blocking_set_len(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        OpenedFileClient::blocking_set_len(self, len)
    }
    async fn io(
        &self,
    ) -> std::result::Result<
        std::sync::Arc<dyn FileIO + Send + Sync + 'static>,
        wasm_bus::abi::CallError,
    > {
        OpenedFileClient::io(self).await
    }
    fn blocking_io(
        &self,
    ) -> std::result::Result<
        std::sync::Arc<dyn FileIO + Send + Sync + 'static>,
        wasm_bus::abi::CallError,
    > {
        OpenedFileClient::blocking_io(self)
    }
    fn as_client(&self) -> Option<OpenedFileClient> {
        Some(self.clone())
    }
    fn handle(&self) -> Option<wasm_bus::abi::CallHandle> {
        OpenedFileClient::handle(self)
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
#[async_trait::async_trait]
pub trait FileIO
where
    Self: std::fmt::Debug + Send + Sync,
{
    async fn seek(
        &self,
        from: SeekFrom,
    ) -> std::result::Result<FsResult<u64>, wasm_bus::abi::CallError>;
    async fn flush(&self) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError>;
    async fn write(
        &self,
        data: Vec<u8>,
    ) -> std::result::Result<FsResult<u64>, wasm_bus::abi::CallError>;
    async fn read(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<Vec<u8>>, wasm_bus::abi::CallError>;
    fn blocking_seek(
        &self,
        from: SeekFrom,
    ) -> std::result::Result<FsResult<u64>, wasm_bus::abi::CallError>;
    fn blocking_flush(&self) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError>;
    fn blocking_write(
        &self,
        data: Vec<u8>,
    ) -> std::result::Result<FsResult<u64>, wasm_bus::abi::CallError>;
    fn blocking_read(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<Vec<u8>>, wasm_bus::abi::CallError>;
    fn as_client(&self) -> Option<FileIOClient>;
    fn handle(&self) -> Option<wasm_bus::abi::CallHandle>;
}
#[async_trait::async_trait]
pub trait FileIOSimplified
where
    Self: std::fmt::Debug + Send + Sync,
{
    async fn seek(&self, from: SeekFrom) -> FsResult<u64>;
    async fn flush(&self) -> FsResult<()>;
    async fn write(&self, data: Vec<u8>) -> FsResult<u64>;
    async fn read(&self, len: u64) -> FsResult<Vec<u8>>;
}
#[async_trait::async_trait]
impl<T> FileIO for T
where
    T: FileIOSimplified,
{
    async fn seek(
        &self,
        from: SeekFrom,
    ) -> std::result::Result<FsResult<u64>, wasm_bus::abi::CallError> {
        Ok(FileIOSimplified::seek(self, from).await)
    }
    fn blocking_seek(
        &self,
        from: SeekFrom,
    ) -> std::result::Result<FsResult<u64>, wasm_bus::abi::CallError> {
        Ok(wasm_bus::task::block_on(FileIOSimplified::seek(self, from)))
    }
    async fn flush(&self) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        Ok(FileIOSimplified::flush(self).await)
    }
    fn blocking_flush(&self) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        Ok(wasm_bus::task::block_on(FileIOSimplified::flush(self)))
    }
    async fn write(
        &self,
        data: Vec<u8>,
    ) -> std::result::Result<FsResult<u64>, wasm_bus::abi::CallError> {
        Ok(FileIOSimplified::write(self, data).await)
    }
    fn blocking_write(
        &self,
        data: Vec<u8>,
    ) -> std::result::Result<FsResult<u64>, wasm_bus::abi::CallError> {
        Ok(wasm_bus::task::block_on(FileIOSimplified::write(
            self, data,
        )))
    }
    async fn read(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<Vec<u8>>, wasm_bus::abi::CallError> {
        Ok(FileIOSimplified::read(self, len).await)
    }
    fn blocking_read(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<Vec<u8>>, wasm_bus::abi::CallError> {
        Ok(wasm_bus::task::block_on(FileIOSimplified::read(self, len)))
    }
    fn as_client(&self) -> Option<FileIOClient> {
        None
    }
    fn handle(&self) -> Option<wasm_bus::abi::CallHandle> {
        None
    }
}
#[derive(Debug, Clone)]
pub struct FileIOService {}
impl FileIOService {
    #[allow(dead_code)]
    pub(crate) fn attach(
        wasm_me: std::sync::Arc<dyn FileIO + Send + Sync + 'static>,
        call_handle: wasm_bus::abi::CallHandle,
    ) {
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::respond_to(
                call_handle,
                wasm_bus::abi::SerializationFormat::Bincode,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle, wasm_req: FileIoSeekRequest| {
                    let wasm_me = wasm_me.clone();
                    let from = wasm_req.from;
                    async move { wasm_me.seek(from).await }
                },
                false,
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::respond_to(
                call_handle,
                wasm_bus::abi::SerializationFormat::Bincode,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle, wasm_req: FileIoFlushRequest| {
                    let wasm_me = wasm_me.clone();
                    async move { wasm_me.flush().await }
                },
                false,
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::respond_to(
                call_handle,
                wasm_bus::abi::SerializationFormat::Bincode,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle, wasm_req: FileIoWriteRequest| {
                    let wasm_me = wasm_me.clone();
                    let data = wasm_req.data;
                    async move { wasm_me.write(data).await }
                },
                false,
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::respond_to(
                call_handle,
                wasm_bus::abi::SerializationFormat::Bincode,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle, wasm_req: FileIoReadRequest| {
                    let wasm_me = wasm_me.clone();
                    let len = wasm_req.len;
                    async move { wasm_me.read(len).await }
                },
                false,
            );
        }
    }
    pub fn listen(wasm_me: std::sync::Arc<dyn FileIO + Send + Sync + 'static>) {
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::listen(
                wasm_bus::abi::SerializationFormat::Bincode,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle, wasm_req: FileIoSeekRequest| {
                    let wasm_me = wasm_me.clone();
                    let from = wasm_req.from;
                    async move { wasm_me.seek(from).await }
                },
                false,
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::listen(
                wasm_bus::abi::SerializationFormat::Bincode,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle, wasm_req: FileIoFlushRequest| {
                    let wasm_me = wasm_me.clone();
                    async move { wasm_me.flush().await }
                },
                false,
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::listen(
                wasm_bus::abi::SerializationFormat::Bincode,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle, wasm_req: FileIoWriteRequest| {
                    let wasm_me = wasm_me.clone();
                    let data = wasm_req.data;
                    async move { wasm_me.write(data).await }
                },
                false,
            );
        }
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::listen(
                wasm_bus::abi::SerializationFormat::Bincode,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle, wasm_req: FileIoReadRequest| {
                    let wasm_me = wasm_me.clone();
                    let len = wasm_req.len;
                    async move { wasm_me.read(len).await }
                },
                false,
            );
        }
    }
    pub fn serve() {
        wasm_bus::task::serve();
    }
}
#[derive(Debug, Clone)]
pub struct FileIOClient {
    wapm: std::borrow::Cow<'static, str>,
    instance: Option<wasm_bus::abi::CallInstance>,
    parent: Option<std::sync::Arc<wasm_bus::abi::DetachedCall<()>>>,
    task: Option<wasm_bus::abi::Call>,
    join: Option<wasm_bus::abi::CallJoin<()>>,
}
impl FileIOClient {
    pub fn new(wapm: &str) -> Self {
        Self {
            wapm: wapm.to_string().into(),
            instance: None,
            parent: None,
            task: None,
            join: None,
        }
    }
    pub fn new_with_instance(wapm: &str, instance: &str, access_token: &str) -> Self {
        Self {
            wapm: wapm.to_string().into(),
            instance: Some(wasm_bus::abi::CallInstance::new(instance, access_token)),
            parent: None,
            task: None,
            join: None,
        }
    }
    pub fn attach(task: wasm_bus::abi::DetachedCall<()>) -> Self {
        let wapm = task.wapm();
        let instance = task.clone_instance();
        Self {
            wapm,
            instance,
            parent: Some(std::sync::Arc::new(task)),
            task: None,
            join: None,
        }
    }
    pub fn id(&self) -> u32 {
        self.task.as_ref().map(|a| a.id()).unwrap_or(0u32)
    }
    pub fn handle(&self) -> Option<wasm_bus::abi::CallHandle> {
        if let Some(handle) = self.task.as_ref().map(|a| a.handle()) {
            return Some(handle);
        }
        None
    }
    pub fn wait(self) -> Result<(), wasm_bus::abi::CallError> {
        if let Some(join) = self.join {
            join.wait()?;
        }
        if let Some(task) = self.task {
            task.join().wait()?;
        }
        Ok(())
    }
    pub fn try_wait(&mut self) -> Result<Option<()>, wasm_bus::abi::CallError> {
        if let Some(task) = self.task.take() {
            self.join.replace(task.join());
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
    ) -> std::result::Result<FsResult<u64>, wasm_bus::abi::CallError> {
        let request = FileIoSeekRequest { from };
        wasm_bus::abi::call_ext(
            self.parent.as_ref().map(|a| a.handle()),
            self.wapm.clone(),
            wasm_bus::abi::SerializationFormat::Bincode,
            self.instance.clone(),
            request,
        )
        .invoke()
        .join()
        .await
    }
    pub async fn flush(&self) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        let request = FileIoFlushRequest {};
        wasm_bus::abi::call_ext(
            self.parent.as_ref().map(|a| a.handle()),
            self.wapm.clone(),
            wasm_bus::abi::SerializationFormat::Bincode,
            self.instance.clone(),
            request,
        )
        .invoke()
        .join()
        .await
    }
    pub async fn write(
        &self,
        data: Vec<u8>,
    ) -> std::result::Result<FsResult<u64>, wasm_bus::abi::CallError> {
        let request = FileIoWriteRequest { data };
        wasm_bus::abi::call_ext(
            self.parent.as_ref().map(|a| a.handle()),
            self.wapm.clone(),
            wasm_bus::abi::SerializationFormat::Bincode,
            self.instance.clone(),
            request,
        )
        .invoke()
        .join()
        .await
    }
    pub async fn read(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<Vec<u8>>, wasm_bus::abi::CallError> {
        let request = FileIoReadRequest { len };
        wasm_bus::abi::call_ext(
            self.parent.as_ref().map(|a| a.handle()),
            self.wapm.clone(),
            wasm_bus::abi::SerializationFormat::Bincode,
            self.instance.clone(),
            request,
        )
        .invoke()
        .join()
        .await
    }
    pub fn blocking_seek(
        &self,
        from: SeekFrom,
    ) -> std::result::Result<FsResult<u64>, wasm_bus::abi::CallError> {
        wasm_bus::task::block_on(self.seek(from))
    }
    pub fn blocking_flush(&self) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        wasm_bus::task::block_on(self.flush())
    }
    pub fn blocking_write(
        &self,
        data: Vec<u8>,
    ) -> std::result::Result<FsResult<u64>, wasm_bus::abi::CallError> {
        wasm_bus::task::block_on(self.write(data))
    }
    pub fn blocking_read(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<Vec<u8>>, wasm_bus::abi::CallError> {
        wasm_bus::task::block_on(self.read(len))
    }
}
impl std::future::Future for FileIOClient {
    type Output = Result<(), wasm_bus::abi::CallError>;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if let Some(task) = self.task.take() {
            self.join.replace(task.join());
        }
        if let Some(join) = self.join.as_mut() {
            let join = std::pin::Pin::new(join);
            return join.poll(cx);
        } else {
            std::task::Poll::Ready(Ok(()))
        }
    }
}
#[async_trait::async_trait]
impl FileIO for FileIOClient {
    async fn seek(
        &self,
        from: SeekFrom,
    ) -> std::result::Result<FsResult<u64>, wasm_bus::abi::CallError> {
        FileIOClient::seek(self, from).await
    }
    fn blocking_seek(
        &self,
        from: SeekFrom,
    ) -> std::result::Result<FsResult<u64>, wasm_bus::abi::CallError> {
        FileIOClient::blocking_seek(self, from)
    }
    async fn flush(&self) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        FileIOClient::flush(self).await
    }
    fn blocking_flush(&self) -> std::result::Result<FsResult<()>, wasm_bus::abi::CallError> {
        FileIOClient::blocking_flush(self)
    }
    async fn write(
        &self,
        data: Vec<u8>,
    ) -> std::result::Result<FsResult<u64>, wasm_bus::abi::CallError> {
        FileIOClient::write(self, data).await
    }
    fn blocking_write(
        &self,
        data: Vec<u8>,
    ) -> std::result::Result<FsResult<u64>, wasm_bus::abi::CallError> {
        FileIOClient::blocking_write(self, data)
    }
    async fn read(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<Vec<u8>>, wasm_bus::abi::CallError> {
        FileIOClient::read(self, len).await
    }
    fn blocking_read(
        &self,
        len: u64,
    ) -> std::result::Result<FsResult<Vec<u8>>, wasm_bus::abi::CallError> {
        FileIOClient::blocking_read(self, len)
    }
    fn as_client(&self) -> Option<FileIOClient> {
        Some(self.clone())
    }
    fn handle(&self) -> Option<wasm_bus::abi::CallHandle> {
        FileIOClient::handle(self)
    }
}
*/