use error_chain::error_chain;
use std::time::Duration;

use ::ate::prelude::*;
use crate::request::*;

error_chain! {
    types {
        SudoError, SudoErrorKind, ResultExt, Result;
    }
    links {
        AteError(::ate::error::AteError, ::ate::error::AteErrorKind);
        ChainCreationError(::ate::error::ChainCreationError, ::ate::error::ChainCreationErrorKind);
        SerializationError(::ate::error::SerializationError, ::ate::error::SerializationErrorKind);
        InvokeError(::ate::error::InvokeError, ::ate::error::InvokeErrorKind);
        LoginError(super::LoginError, super::LoginErrorKind);
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
        MissingToken {
            description("login failed as the token was missing"),
            display("login failed as the token was missing"),
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
        WrongCode {
            description("login failed due to an incorrect authentication code")
            display("login failed due to an incorrect authentication code")
        }
        InternalError(code: u16) {
            description("login failed as the server experienced an internal error")
            display("login failed as the server experienced an internal error - code={}", code)
        }
    }
}

impl From<SudoError>
for AteError
{
    fn from(err: SudoError) -> AteError {
        AteErrorKind::ServiceError(err.to_string()).into()
    }
}

impl From<SudoFailed>
for SudoError {
    fn from(err: SudoFailed) -> SudoError {
        match err {
            SudoFailed::AccountLocked(duration) => SudoErrorKind::AccountLocked(duration).into(),
            SudoFailed::MissingToken => SudoErrorKind::MissingToken.into(),
            SudoFailed::NoMasterKey => SudoErrorKind::NoMasterKey.into(),
            SudoFailed::Unverified(username) => SudoErrorKind::Unverified(username).into(),
            SudoFailed::UserNotFound(username) => SudoErrorKind::NotFound(username).into(),
            SudoFailed::WrongCode => SudoErrorKind::WrongCode.into(),
            SudoFailed::InternalError(code) => SudoErrorKind::InternalError(code).into(),
        }
    }
}