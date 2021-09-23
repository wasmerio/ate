use error_chain::error_chain;

use ::ate::prelude::*;
use crate::request::*;

error_chain! {
    types {
        GroupRemoveError, GroupRemoveErrorKind, ResultExt, Result;
    }
    links {
        QueryError(super::QueryError, super::QueryErrorKind);
        AteError(::ate::error::AteError, ::ate::error::AteErrorKind);
        ChainCreationError(::ate::error::ChainCreationError, ::ate::error::ChainCreationErrorKind);
        SerializationError(::ate::error::SerializationError, ::ate::error::SerializationErrorKind);
        InvokeError(::ate::error::InvokeError, ::ate::error::InvokeErrorKind);
        GatherError(super::GatherError, super::GatherErrorKind);
    }
    foreign_links {
        IO(tokio::io::Error);
    }
    errors {
        NoAccess {
            description("group remove failed as the referrer has no access to this group")
            display("group remove failed as the referrer has no access to this group")
        }
        NoMasterKey {
            description("group remove failed as the server has not been properly initialized")
            display("group remove failed as the server has not been properly initialized")
        }
        GroupNotFound {
            description("group remove failed as the group does not exist")
            display("group remove failed as the group does not exist")
        }
        InternalError(code: u16) {
            description("group remove failed as the server experienced an internal error")
            display("group remove failed as the server experienced an internal error - code={}", code)
        }
    }
}

impl From<GroupRemoveError>
for AteError
{
    fn from(err: GroupRemoveError) -> AteError {
        AteErrorKind::ServiceError(err.to_string()).into()
    }
}

impl From<GroupRemoveFailed>
for GroupRemoveError {
    fn from(err: GroupRemoveFailed) -> GroupRemoveError {
        match err {
            GroupRemoveFailed::GroupNotFound => GroupRemoveErrorKind::GroupNotFound.into(),
            GroupRemoveFailed::NoAccess => GroupRemoveErrorKind::NoAccess.into(),
            GroupRemoveFailed::NoMasterKey => GroupRemoveErrorKind::NoMasterKey.into(),
            GroupRemoveFailed::InternalError(code) => GroupRemoveErrorKind::InternalError(code).into(),
        }
    }
}