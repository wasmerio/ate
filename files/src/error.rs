use ate::error::{AteError, AteErrorKind};
use error_chain::error_chain;

error_chain! {
    types {
        FileSystemError, FileSystemErrorKind, ResultExt, Result;
    }
    links {
        AteError(::ate::error::AteError, ::ate::error::AteErrorKind);
        LoadError(::ate::error::LoadError, ::ate::error::LoadErrorKind);
        SerializationError(::ate::error::SerializationError, ::ate::error::SerializationErrorKind);
        CommitError(::ate::error::CommitError, ::ate::error::CommitErrorKind);
        LoginError(::ate_auth::error::LoginError, ::ate_auth::error::LoginErrorKind);
        CreateError(::ate_auth::error::CreateError, ::ate_auth::error::CreateErrorKind);
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
        MissingPermissions {
            description("missing permissions for this entry"),
            display("missing permissions for this entry")
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

impl From<FileSystemError>
for AteError
{
    fn from(err: FileSystemError) -> AteError {
        AteErrorKind::ServiceError(err.to_string()).into()
    }
}