use error_chain::error_chain;

use ::ate::prelude::*;
use crate::commands::*;

error_chain! {
    types {
        ResetError, ResetErrorKind, ResultExt, Result;
    }
    links {
        AteError(::ate::error::AteError, ::ate::error::AteErrorKind);
        ChainCreationError(::ate::error::ChainCreationError, ::ate::error::ChainCreationErrorKind);
        SerializationError(::ate::error::SerializationError, ::ate::error::SerializationErrorKind);
        InvokeError(::ate::error::InvokeError, ::ate::error::InvokeErrorKind);
        LoginError(super::LoginError, super::LoginErrorKind);
        SudoError(super::SudoError, super::SudoErrorKind);
    }
    foreign_links {
        IO(tokio::io::Error);
    }
    errors {
        NotFound(email: String) {
            description("reset failed as the user does not exist"),
            display("reset failed as the user does not exist ({})", email),
        }
        PasswordMismatch {
            description("reset failed as the passwords did not match")
            display("reset failed as the passwords did not match")
        }
        AuthenticatorCodeEqual {
            description("reset failed as you entered the same authenticator code twice")
            display("reset failed as you entered the same authenticator code twice")
        }
        InvalidRecoveryCode {
            description("the supplied recovery code was not valid")
            display("the supplied recovery code was not valid")
        }
        InvalidAuthenticatorCode {
            description("one or more of the supplied authenticator codes was not valid")
            display("one or more of the supplied authenticator codes was not valid")
        }
        NoMasterKey {
            description("reset failed as the server has not been properly initialized")
            display("reset failed as the server has not been properly initialized")
        }
        InternalError(code: u16) {
            description("reset failed as the server experienced an internal error")
            display("reset failed as the server experienced an internal error - code={}", code)
        }
    }
}

impl From<ResetError>
for AteError
{
    fn from(err: ResetError) -> AteError {
        AteErrorKind::ServiceError(err.to_string()).into()
    }
}

impl From<ResetFailed>
for ResetError {
    fn from(err: ResetFailed) -> ResetError {
        match err {
            ResetFailed::InvalidEmail(email) => ResetErrorKind::NotFound(email).into(),
            ResetFailed::InvalidRecoveryCode => ResetErrorKind::InvalidRecoveryCode.into(),
            ResetFailed::InvalidAuthenticatorCode => ResetErrorKind::InvalidAuthenticatorCode.into(),
            ResetFailed::NoMasterKey => ResetErrorKind::NoMasterKey.into(),
            ResetFailed::InternalError(code) => ResetErrorKind::InternalError(code).into(),
        }
    }
}