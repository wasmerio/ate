use error_chain::error_chain;

use ::ate::prelude::*;
use crate::request::*;

error_chain! {
    types {
        GroupUserAddError, GroupUserAddErrorKind, ResultExt, Result;
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
            description("group user add failed as the role purpose was invalid"),
            display("group user add failed as the role purpose was invalid"),
        }
        UnknownIdentity {
            description("group user add failed as the identity of the caller could not be established")
            display("group user add failed as the identity of the caller could not be established")
        }
        GroupNotFound {
            description("group user add failed as the referenced group does not exist")
            display("group user add failed as the referenced group does not exist")
        }
        NoAccess {
            description("group user add failed as the referrer has no access to this group")
            display("group user add failed as the referrer has no access to this group")
        }
        NoMasterKey {
            description("group user add failed as the server has not been properly initialized")
            display("group user add failed as the server has not been properly initialized")
        }
        InternalError(code: u16) {
            description("group user add failed as the server experienced an internal error")
            display("group user add failed as the server experienced an internal error - code={}", code)
        }
    }
}

impl From<GroupUserAddError>
for AteError
{
    fn from(err: GroupUserAddError) -> AteError {
        AteErrorKind::ServiceError(err.to_string()).into()
    }
}

impl From<GroupUserAddFailed>
for GroupUserAddError {
    fn from(err: GroupUserAddFailed) -> GroupUserAddError {
        match err {
            GroupUserAddFailed::GroupNotFound => GroupUserAddErrorKind::GroupNotFound.into(),
            GroupUserAddFailed::NoAccess => GroupUserAddErrorKind::NoAccess.into(),
            GroupUserAddFailed::NoMasterKey => GroupUserAddErrorKind::NoMasterKey.into(),
            GroupUserAddFailed::UnknownIdentity => GroupUserAddErrorKind::UnknownIdentity.into(),
            GroupUserAddFailed::InvalidPurpose => GroupUserAddErrorKind::InvalidPurpose.into(),
            GroupUserAddFailed::InternalError(code) => GroupUserAddErrorKind::InternalError(code).into(),
        }
    }
}