use error_chain::error_chain;

use ::ate::prelude::*;
use crate::commands::*;

error_chain! {
    types {
        LoginError, LoginErrorKind, ResultExt, Result;
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
        NoMasterKey {
            description("login failed as the server has not been properly initialized")
            display("login failed as the server has not been properly initialized")
        }
        Timeout {
            description("login failed due to a timeout"),
            display("login failed due to a timeout"),
        }
        NotFound {
            description("login failed as the account does not exist"),
            display("login failed as the account does not exist"),
        }
        AccountLocked {
            description("login failed as the account is locked")
            display("login failed as the account is locked")
        }
        Unverified {
            description("login failed as the account is not yet verified")
            display("login failed as the account is not yet verified")
        }
        WrongPasswordOrCode {
            description("login failed due to an incorrect password or authentication code")
            display("login failed due to an incorrect password or authentication code")
        }
        InternalError(code: u16) {
            description("login failed as the server experienced an internal error")
            display("login failed as the server experienced an internal error - code={}", code)
        }
    }
}

impl From<LoginError>
for AteError
{
    fn from(err: LoginError) -> AteError {
        AteErrorKind::ServiceError(err.to_string()).into()
    }
}

impl From<LoginFailed>
for LoginError {
    fn from(err: LoginFailed) -> LoginError {
        match err {
            LoginFailed::AccountLocked => LoginErrorKind::AccountLocked.into(),
            LoginFailed::NoMasterKey => LoginErrorKind::NoMasterKey.into(),
            LoginFailed::Unverified => LoginErrorKind::Unverified.into(),
            LoginFailed::UserNotFound => LoginErrorKind::NotFound.into(),
            LoginFailed::WrongPasswordOrCode => LoginErrorKind::WrongPasswordOrCode.into(),
            LoginFailed::InternalError(code) => LoginErrorKind::InternalError(code).into(),
        }
    }
}