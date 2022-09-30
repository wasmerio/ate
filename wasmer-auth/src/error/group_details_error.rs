use error_chain::error_chain;

use crate::request::*;
use ::ate::prelude::*;

error_chain! {
    types {
        GroupDetailsError, GroupDetailsErrorKind, ResultExt, Result;
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
        GroupNotFound {
            description("group details failed as the group does not exist")
            display("group details failed as the group does not exist")
        }
        NoAccess {
            description("group details failed as the referrer has no access to this group")
            display("group details failed as the referrer has no access to this group")
        }
        NoMasterKey {
            description("group details failed as the server has not been properly initialized")
            display("group deatils failed as the server has not been properly initialized")
        }
        InternalError(code: u16) {
            description("group details failed as the server experienced an internal error")
            display("group details failed as the server experienced an internal error - code={}", code)
        }
    }
}

impl From<GroupDetailsError> for AteError {
    fn from(err: GroupDetailsError) -> AteError {
        AteErrorKind::ServiceError(err.to_string()).into()
    }
}

impl From<GroupDetailsFailed> for GroupDetailsError {
    fn from(err: GroupDetailsFailed) -> GroupDetailsError {
        match err {
            GroupDetailsFailed::GroupNotFound => GroupDetailsErrorKind::GroupNotFound.into(),
            GroupDetailsFailed::NoAccess => GroupDetailsErrorKind::NoAccess.into(),
            GroupDetailsFailed::NoMasterKey => GroupDetailsErrorKind::NoMasterKey.into(),
            GroupDetailsFailed::InternalError(code) => {
                GroupDetailsErrorKind::InternalError(code).into()
            }
        }
    }
}
