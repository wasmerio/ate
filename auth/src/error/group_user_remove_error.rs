use error_chain::error_chain;

use ::ate::prelude::*;
use crate::commands::*;

error_chain! {
    types {
        GroupUserRemoveError, GroupUserRemoveErrorKind, ResultExt, Result;
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
        InvalidPurpose {
            description("group user remove failed as the role purpose was invalid"),
            display("group user remove failed as the role purpose was invalid"),
        }
        NoAccess {
            description("group user remove failed as the referrer has no access to this group")
            display("group user remove failed as the referrer has no access to this group")
        }
        NoMasterKey {
            description("group user remove failed as the server has not been properly initialized")
            display("group user remove failed as the server has not been properly initialized")
        }
        GroupNotFound {
            description("group user remove failed as the group does not exist")
            display("group user remove failed as the group does not exist")
        }
        RoleNotFound {
            description("group user remove failed as the group role does not exist")
            display("group user remove failed as the group role does not exist")
        }
        NothingToRemove {
            description("group user remove failed as the user is not a member of this group role")
            display("group user remove failed as the user is not a member of this group role")
        }
        InternalError(code: u16) {
            description("group user remove failed as the server experienced an internal error")
            display("group user remove failed as the server experienced an internal error - code={}", code)
        }
    }
}

impl From<GroupUserRemoveError>
for AteError
{
    fn from(err: GroupUserRemoveError) -> AteError {
        AteErrorKind::ServiceError(err.to_string()).into()
    }
}

impl From<GroupUserRemoveFailed>
for GroupUserRemoveError {
    fn from(err: GroupUserRemoveFailed) -> GroupUserRemoveError {
        match err {
            GroupUserRemoveFailed::GroupNotFound => GroupUserRemoveErrorKind::GroupNotFound.into(),
            GroupUserRemoveFailed::NoAccess => GroupUserRemoveErrorKind::NoAccess.into(),
            GroupUserRemoveFailed::NoMasterKey => GroupUserRemoveErrorKind::NoMasterKey.into(),
            GroupUserRemoveFailed::NothingToRemove => GroupUserRemoveErrorKind::NothingToRemove.into(),
            GroupUserRemoveFailed::RoleNotFound => GroupUserRemoveErrorKind::RoleNotFound.into(),
            GroupUserRemoveFailed::InternalError(code) => GroupUserRemoveErrorKind::InternalError(code).into(),
        }
    }
}