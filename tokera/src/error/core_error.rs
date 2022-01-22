use error_chain::error_chain;

use crate::request::*;

error_chain! {
    types {
        CoreError, CoreErrorKind, ResultExt, Result;
    }
    links {
        AteError(::ate::error::AteError, ::ate::error::AteErrorKind);
        ChainCreationError(::ate::error::ChainCreationError, ::ate::error::ChainCreationErrorKind);
        SerializationError(::ate::error::SerializationError, ::ate::error::SerializationErrorKind);
        InvokeError(::ate::error::InvokeError, ::ate::error::InvokeErrorKind);
        TimeError(::ate::error::TimeError, ::ate::error::TimeErrorKind);
        LoadError(::ate::error::LoadError, ::ate::error::LoadErrorKind);
        CommitError(::ate::error::CommitError, ::ate::error::CommitErrorKind);
        LockError(::ate::error::LockError, ::ate::error::LockErrorKind);
    }
    foreign_links {
        IO(tokio::io::Error);
    }
    errors {
        OperatorBanned {
            description("the operator is currently banned"),
            display("the operator is currently banned"),
        }
        OperatorNotFound {
            description("the operator could not be found"),
            display("the operator could not be found"),
        }
        AccountSuspended {
            description("the account is currently suspended"),
            display("the account is currently suspended"),
        }
        AuthenticationFailed {
            description("the caller has no authentication to this coin"),
            display("the caller has no authentication to this coin"),
        }
        NoMasterKey {
            description("this server has not been properly initialized (master key)"),
            display("this server has not been properly initialized (master key)"),
        }
        Forbidden {
            description("this operation is forbidden"),
            display("this operation is forbidden"),
        }
        MissingTokenIdentity {
            description("supplied token is missing an identity"),
            display("supplied token is missing an identity"),
        }
        MissingTokenKey {
            description("supplied token is missing an authentication key"),
            display("supplied token is missing an authentication key"),
        }
        MissingBrokerKey {
            description("the caller did not provider a broker key"),
            display("the caller did not provider a broker key"),
        }
        NoPayPalConfig {
            description("this server has not been properly initialized (paypal config)"),
            display("this server has not been properly initialized (paypal config)"),
        }
        SafetyCheckFailed {
            description("one of the saftey and security failsafes was triggered"),
            display("one of the saftey and security failsafes was triggered"),
        }
        InternalError(code: u16) {
            description("the server experienced an internal error")
            display("the server experienced an internal error - code={}", code)
        }
        Other(err: String) {
            description("this server experienced an error"),
            display("this server experienced an error - {}", err),
        }
    }
}

impl From<ServiceFindFailed> for CoreError {
    fn from(err: ServiceFindFailed) -> CoreError {
        match err {
            ServiceFindFailed::Forbidden => CoreErrorKind::Forbidden.into(),
            ServiceFindFailed::InternalError(code) => CoreErrorKind::InternalError(code).into(),
        }
    }
}