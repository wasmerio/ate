use error_chain::error_chain;

use crate::request::*;
use super::*;

error_chain! {
    types {
        InstanceError, InstanceErrorKind, ResultExt, Result;
    }
    links {
        CoreError(super::CoreError, super::CoreErrorKind);
        QueryError(super::QueryError, super::QueryErrorKind);
    }
    foreign_links {
        IO(tokio::io::Error);
    }
    errors {
        InvalidInstance {
            description("the service was this code could not be found")
            display("the service was this code could not be found")
        }
        Unsupported {
            description("the operation is not yet supported")
            display("the operation is not yet supported")
        }
    }
}

impl From<::ate::error::AteError> for InstanceError {
    fn from(err: ::ate::error::AteError) -> Self {
        InstanceErrorKind::CoreError(CoreErrorKind::AteError(err.0)).into()
    }
}

impl From<::ate::error::AteErrorKind> for InstanceErrorKind {
    fn from(err: ::ate::error::AteErrorKind) -> Self {
        InstanceErrorKind::CoreError(CoreErrorKind::AteError(err))
    }
}

impl From<::ate::error::ChainCreationError> for InstanceError {
    fn from(err: ::ate::error::ChainCreationError) -> Self {
        InstanceErrorKind::CoreError(CoreErrorKind::ChainCreationError(err.0)).into()
    }
}

impl From<::ate::error::ChainCreationErrorKind> for InstanceErrorKind {
    fn from(err: ::ate::error::ChainCreationErrorKind) -> Self {
        InstanceErrorKind::CoreError(CoreErrorKind::ChainCreationError(err))
    }
}

impl From<::ate::error::SerializationError> for InstanceError {
    fn from(err: ::ate::error::SerializationError) -> Self {
        InstanceErrorKind::CoreError(CoreErrorKind::SerializationError(err.0)).into()
    }
}

impl From<::ate::error::SerializationErrorKind> for InstanceErrorKind {
    fn from(err: ::ate::error::SerializationErrorKind) -> Self {
        InstanceErrorKind::CoreError(CoreErrorKind::SerializationError(err))
    }
}

impl From<::ate::error::InvokeError> for InstanceError {
    fn from(err: ::ate::error::InvokeError) -> Self {
        InstanceErrorKind::CoreError(CoreErrorKind::InvokeError(err.0)).into()
    }
}

impl From<::ate::error::InvokeErrorKind> for InstanceErrorKind {
    fn from(err: ::ate::error::InvokeErrorKind) -> Self {
        InstanceErrorKind::CoreError(CoreErrorKind::InvokeError(err))
    }
}

impl From<::ate::error::TimeError> for InstanceError {
    fn from(err: ::ate::error::TimeError) -> Self {
        InstanceErrorKind::CoreError(CoreErrorKind::TimeError(err.0)).into()
    }
}

impl From<::ate::error::TimeErrorKind> for InstanceErrorKind {
    fn from(err: ::ate::error::TimeErrorKind) -> Self {
        InstanceErrorKind::CoreError(CoreErrorKind::TimeError(err))
    }
}

impl From<::ate::error::LoadError> for InstanceError {
    fn from(err: ::ate::error::LoadError) -> Self {
        InstanceErrorKind::CoreError(CoreErrorKind::LoadError(err.0)).into()
    }
}

impl From<::ate::error::LoadErrorKind> for InstanceErrorKind {
    fn from(err: ::ate::error::LoadErrorKind) -> Self {
        InstanceErrorKind::CoreError(CoreErrorKind::LoadError(err))
    }
}

impl From<::ate::error::CommitError> for InstanceError {
    fn from(err: ::ate::error::CommitError) -> Self {
        InstanceErrorKind::CoreError(CoreErrorKind::CommitError(err.0)).into()
    }
}

impl From<::ate::error::CommitErrorKind> for InstanceErrorKind {
    fn from(err: ::ate::error::CommitErrorKind) -> Self {
        InstanceErrorKind::CoreError(CoreErrorKind::CommitError(err))
    }
}

impl From<::ate::error::LockError> for InstanceError {
    fn from(err: ::ate::error::LockError) -> Self {
        InstanceErrorKind::CoreError(CoreErrorKind::LockError(err.0)).into()
    }
}

impl From<::ate::error::LockErrorKind> for InstanceErrorKind {
    fn from(err: ::ate::error::LockErrorKind) -> Self {
        InstanceErrorKind::CoreError(CoreErrorKind::LockError(err))
    }
}

impl From<InstanceFindFailed> for InstanceError {
    fn from(err: InstanceFindFailed) -> InstanceError {
        match err {
            InstanceFindFailed::Forbidden => {
                InstanceErrorKind::CoreError(CoreErrorKind::Forbidden).into()
            }
            InstanceFindFailed::InternalError(code) => {
                InstanceErrorKind::CoreError(CoreErrorKind::InternalError(code)).into()
            }
        }
    }
}
