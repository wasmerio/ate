use error_chain::error_chain;

use ::ate::prelude::*;
use crate::commands::*;

error_chain! {
    types {
        GatherError, GatherErrorKind, ResultExt, Result;
    }
    links {
        AteError(::ate::error::AteError, ::ate::error::AteErrorKind);
        ChainCreationError(::ate::error::ChainCreationError, ::ate::error::ChainCreationErrorKind);
        SerializationError(::ate::error::SerializationError, ::ate::error::SerializationErrorKind);
        InvokeError(::ate::error::InvokeError, ::ate::error::InvokeErrorKind);
    }
    foreign_links {
        IO(tokio::io::Error);
    }
    errors {
        Timeout {
            description("login failed due to a timeout")
            display("login failed due to a timeout")
        }
        NotFound(group: String) {
            description("gather failed as the group does not exist"),
            display("gather failed as the group does not exist ({})", group),
        }
        NoAccess {
            description("gather failed as the session has no access to this group")
            display("gather failed as the session has no access to this group")
        }
        NoMasterKey {
            description("gather failed as the server has not been properly initialized")
            display("gather failed as the server has not been properly initialized")
        }
        InternalError(code: u16) {
            description("gather failed as the server experienced an internal error")
            display("gather failed as the server experienced an internal error - code={}", code)
        }
    }
}

impl From<GatherError>
for AteError
{
    fn from(err: GatherError) -> AteError {
        AteErrorKind::ServiceError(err.to_string()).into()
    }
}

impl From<GatherFailed>
for GatherError {
    fn from(err: GatherFailed) -> GatherError {
        match err {
            GatherFailed::GroupNotFound(group) => GatherErrorKind::NotFound(group).into(),
            GatherFailed::NoAccess => GatherErrorKind::NoAccess.into(),
            GatherFailed::NoMasterKey => GatherErrorKind::NoMasterKey.into(),
            GatherFailed::InternalError(code) => GatherErrorKind::InternalError(code).into(),
        }
    }
}