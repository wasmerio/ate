use error_chain::error_chain;

use ate::prelude::*;
use crate::commands::*;

error_chain! {
    types {
        CreateError, CreateErrorKind, ResultExt, Result;
    }
    links {
        QueryError(super::QueryError, super::QueryErrorKind);
        AteError(::ate::error::AteError, ::ate::error::AteErrorKind);
        ChainCreationError(::ate::error::ChainCreationError, ::ate::error::ChainCreationErrorKind);
        SerializationError(::ate::error::SerializationError, ::ate::error::SerializationErrorKind);
        InvokeError(::ate::error::InvokeError, ::ate::error::InvokeErrorKind);
    }
    foreign_links {
        IO(tokio::io::Error);
    }
    errors {
        OperatorBanned {
            description("create failed as the operator is currently banned")
            display("create failed as the operator is currently banned")
        }
        OperatorNotFound {
            description("create failed as the operator could not be found")
            display("create failed as the operator could not be found")
        }
        AccountSuspended {
            description("create failed as the account is currently suspended")
            display("create failed as the account is currently suspended")
        }
        MissingReadKey {
            description("create failed as the session is missing a read key")
            display("create failed as the session is missing a read key")
        }
        PasswordMismatch {
            description("create failed as the passwords did not match")
            display("create failed as the passwords did not match")
        }
        AlreadyExists(msg: String) {
            description("create failed as the account or group already exists")
            display("create failed - {}", msg)
        }
        InvalidEmail {
            description("create failed as the email address is invalid")
            display("create failed as the email address is invalid")
        }
        NoMoreRoom {
            description("create failed as the account or group as there is no more room - try a different name")
            display("create failed as the account or group as there is no more room - try a different name")
        }
        InvalidName(msg: String) {
            description("create failed as the account or group name is invalid")
            display("create failed - {}", msg)
        }
        NoMasterKey {
            description("create failed as the server has not been properly initialized")
            display("create failed as the server has not been properly initialized")
        }
        ValidationError(reason: String) {
            description("create failed as there was a validation error")
            display("create failed as there was a validation error - {}", reason)
        }
        TermsAndConditions(terms: String) {
            description("create failed as the caller did not agree to the terms and conditions")
            display("create failed as the caller did not agree to the terms and conditions")
        }
        InternalError(code: u16) {
            description("create failed as the server experienced an internal error")
            display("create failed as the server experienced an internal error - code={}", code)
        }
    }
}

impl From<CreateError>
for AteError
{
    fn from(err: CreateError) -> AteError {
        AteErrorKind::ServiceError(err.to_string()).into()
    }
}

impl From<CreateGroupFailed>
for CreateError {
    fn from(err: CreateGroupFailed) -> CreateError {
        match err {
            CreateGroupFailed::OperatorBanned => CreateErrorKind::OperatorBanned.into(),
            CreateGroupFailed::OperatorNotFound => CreateErrorKind::OperatorNotFound.into(),
            CreateGroupFailed::AccountSuspended => CreateErrorKind::AccountSuspended.into(),
            CreateGroupFailed::AlreadyExists(msg) => CreateErrorKind::AlreadyExists(msg).into(),
            CreateGroupFailed::NoMoreRoom => CreateErrorKind::NoMoreRoom.into(),
            CreateGroupFailed::NoMasterKey => CreateErrorKind::NoMasterKey.into(),
            CreateGroupFailed::InvalidGroupName(msg) => CreateErrorKind::InvalidName(msg).into(),
            CreateGroupFailed::ValidationError(reason) => CreateErrorKind::ValidationError(reason).into(),
            CreateGroupFailed::InternalError(code) => CreateErrorKind::InternalError(code).into(),
        }
    }
}

impl From<CreateUserFailed>
for CreateError {
    fn from(err: CreateUserFailed) -> CreateError {
        match err {
            CreateUserFailed::AlreadyExists(msg) => CreateErrorKind::AlreadyExists(msg).into(),
            CreateUserFailed::InvalidEmail => CreateErrorKind::InvalidEmail.into(),
            CreateUserFailed::NoMasterKey => CreateErrorKind::NoMasterKey.into(),
            CreateUserFailed::NoMoreRoom => CreateErrorKind::NoMoreRoom.into(),
            CreateUserFailed::InternalError(code) => CreateErrorKind::InternalError(code).into(),
            CreateUserFailed::TermsAndConditions(terms) => CreateErrorKind::TermsAndConditions(terms).into(),
        }
    }
}