use error_chain::error_chain;
use std::time::Duration;

use ::ate::prelude::*;
use crate::request::*;

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
        InvalidArguments {
            description("you did not provide the right type or quantity of arguments")
            display("you did not provide the right type or quantity of arguments")
        }
        Timeout {
            description("login failed due to a timeout"),
            display("login failed due to a timeout"),
        }
        NotFound(username: String) {
            description("login failed as the account does not exist"),
            display("login failed for {} as the account does not exist", username),
        }
        AccountLocked(duration: Duration) {
            description("login failed as the account is locked"),
            display("login failed as the account is locked for {} hours", (duration.as_secs() as f32 / 3600f32)),
        }
        Unverified(username: String) {
            description("login failed as the account is not yet verified")
            display("login failed for {} as the account is not yet verified", username)
        }
        WrongPassword {
            description("login failed due to an incorrect password")
            display("login failed due to an incorrect password")
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
            LoginFailed::AccountLocked(duration) => LoginErrorKind::AccountLocked(duration).into(),
            LoginFailed::NoMasterKey => LoginErrorKind::NoMasterKey.into(),
            LoginFailed::Unverified(username) => LoginErrorKind::Unverified(username).into(),
            LoginFailed::UserNotFound(username) => LoginErrorKind::NotFound(username).into(),
            LoginFailed::WrongPassword => LoginErrorKind::WrongPassword.into(),
            LoginFailed::InternalError(code) => LoginErrorKind::InternalError(code).into(),
        }
    }
}