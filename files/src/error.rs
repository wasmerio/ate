use ate::error::*;
use error_chain::error_chain;

error_chain! {
    types {
        FileSystemError, FileSystemErrorKind, ResultExt, Result;
    }
    links {
        AteError(::ate::error::AteError, ::ate::error::AteErrorKind);
        LoginError(::ate_auth::error::LoginError, ::ate_auth::error::LoginErrorKind);
        CreateError(::ate_auth::error::CreateError, ::ate_auth::error::CreateErrorKind);
        GatherError(::ate_auth::error::GatherError, ::ate_auth::error::GatherErrorKind);
    }
    foreign_links {
        IO(tokio::io::Error);
    }
    errors {
        NotDirectory {
            description("the entry is not a directory")
            display("the entry is not a directory")
        }
        IsDirectory {
            description("the entry is a directory")
            display("the entry is a directory")
        }
        DoesNotExist {
            description("the entry does not exist")
            display("the entry does not exist")
        }
        NoAccess {
            description("no access allowed to this entry"),
            display("no access allowed to this entry")
        }
        PermissionDenied {
            description("missing permissions for this operation"),
            display("missing permissions for this operation")
        }
        ReadOnly {
            description("read only"),
            display("read only")
        }
        InvalidArguments {
            description("invalid arguments were supplied"),
            display("invalid arguments were supplied")
        }
        NoEntry {
            description("the entry does not exist"),
            display("the entry does not exist"),
        }
        AlreadyExists {
            description("an entry with this name already exists")
            display("an entry with this name already exists")
        }
        NotImplemented {
            description("the function is not implemented"),
            display("the function is not implemented")
        }
    }
}

impl From<LoadError>
for FileSystemError
{
    fn from(err: LoadError) -> FileSystemError {
        FileSystemError::from_kind(err.0.into())
    }
}

impl From<LoadErrorKind>
for FileSystemErrorKind
{
    fn from(err: LoadErrorKind) -> FileSystemErrorKind {
        match err {
            LoadErrorKind::NotFound(_) => FileSystemErrorKind::NoEntry,
            LoadErrorKind::SerializationError(err) => err.into(),
            LoadErrorKind::TransformationError(err) => err.into(),
            err => FileSystemErrorKind::AteError(AteErrorKind::LoadError(err))
        }
    }
}

impl From<TransformError>
for FileSystemError
{
    fn from(err: TransformError) -> FileSystemError {
        FileSystemError::from_kind(err.0.into())
    }
}

impl From<TransformErrorKind>
for FileSystemErrorKind
{
    fn from(err: TransformErrorKind) -> FileSystemErrorKind {
        match err {
            TransformErrorKind::MissingReadKey(_) => FileSystemErrorKind::NoAccess,
            err => FileSystemErrorKind::AteError(AteErrorKind::TransformError(err))
        }
    }
}

impl From<SerializationError>
for FileSystemError
{
    fn from(err: SerializationError) -> FileSystemError {
        FileSystemError::from_kind(err.0.into())
    }
}

impl From<SerializationErrorKind>
for FileSystemErrorKind
{
    fn from(err: SerializationErrorKind) -> FileSystemErrorKind {
        match err {
            err => FileSystemErrorKind::AteError(AteErrorKind::SerializationError(err))
        }
    }
}

impl From<CommitError>
for FileSystemError
{
    fn from(err: CommitError) -> FileSystemError {
        FileSystemError::from_kind(err.0.into())
    }
}

impl From<CommitErrorKind>
for FileSystemErrorKind
{
    fn from(err: CommitErrorKind) -> FileSystemErrorKind {
        match err {
            CommitErrorKind::CommsError(CommsErrorKind::ReadOnly) => FileSystemErrorKind::NoAccess,
            CommitErrorKind::ReadOnly => FileSystemErrorKind::NoAccess,
            CommitErrorKind::SerializationError(err) => err.into(),
            CommitErrorKind::TransformError(err) => err.into(),
            err => FileSystemErrorKind::AteError(AteErrorKind::CommitError(err))
        }
    }
}

impl From<FileSystemError>
for AteError
{
    fn from(err: FileSystemError) -> AteError {
        AteErrorKind::ServiceError(err.to_string()).into()
    }
}