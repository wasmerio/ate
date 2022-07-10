use error_chain::error_chain;

use crate::model::*;
use crate::request::*;
use ate::prelude::*;

use super::*;

error_chain! {
    types {
        ContractError, ContractErrorKind, ResultExt, Result;
    }
    links {
        CoreError(super::CoreError, super::CoreErrorKind);
        QueryError(super::QueryError, super::QueryErrorKind);
    }
    foreign_links {
        IO(tokio::io::Error);
    }
    errors {
        InvalidService {
            description("service with this code could not be found")
            display("service with this code could not be found")
        }
        UnsupportedCurrency(currency: NationalCurrency) {
            description("service does not support this currency")
            display("service does not support this currency ({})", currency)
        }
        AlreadyExists(msg: String) {
            description("the contract already exists")
            display("{}", msg)
        }
        InvalidReference(reference_number: String) {
            description("invalid reference number"),
            display("invalid reference number ({})", reference_number),
        }
    }
}

impl From<ContractError> for AteError {
    fn from(err: ContractError) -> AteError {
        AteErrorKind::ServiceError(err.to_string()).into()
    }
}

impl From<::ate::error::AteError> for ContractError {
    fn from(err: ::ate::error::AteError) -> Self {
        ContractErrorKind::CoreError(CoreErrorKind::AteError(err.0)).into()
    }
}

impl From<::ate::error::AteErrorKind> for ContractErrorKind {
    fn from(err: ::ate::error::AteErrorKind) -> Self {
        ContractErrorKind::CoreError(CoreErrorKind::AteError(err))
    }
}

impl From<::ate::error::ChainCreationError> for ContractError {
    fn from(err: ::ate::error::ChainCreationError) -> Self {
        ContractErrorKind::CoreError(CoreErrorKind::ChainCreationError(err.0)).into()
    }
}

impl From<::ate::error::ChainCreationErrorKind> for ContractErrorKind {
    fn from(err: ::ate::error::ChainCreationErrorKind) -> Self {
        ContractErrorKind::CoreError(CoreErrorKind::ChainCreationError(err))
    }
}

impl From<::ate::error::SerializationError> for ContractError {
    fn from(err: ::ate::error::SerializationError) -> Self {
        ContractErrorKind::CoreError(CoreErrorKind::SerializationError(err.0)).into()
    }
}

impl From<::ate::error::SerializationErrorKind> for ContractErrorKind {
    fn from(err: ::ate::error::SerializationErrorKind) -> Self {
        ContractErrorKind::CoreError(CoreErrorKind::SerializationError(err))
    }
}

impl From<::ate::error::InvokeError> for ContractError {
    fn from(err: ::ate::error::InvokeError) -> Self {
        ContractErrorKind::CoreError(CoreErrorKind::InvokeError(err.0)).into()
    }
}

impl From<::ate::error::InvokeErrorKind> for ContractErrorKind {
    fn from(err: ::ate::error::InvokeErrorKind) -> Self {
        ContractErrorKind::CoreError(CoreErrorKind::InvokeError(err))
    }
}

impl From<::ate::error::TimeError> for ContractError {
    fn from(err: ::ate::error::TimeError) -> Self {
        ContractErrorKind::CoreError(CoreErrorKind::TimeError(err.0)).into()
    }
}

impl From<::ate::error::TimeErrorKind> for ContractErrorKind {
    fn from(err: ::ate::error::TimeErrorKind) -> Self {
        ContractErrorKind::CoreError(CoreErrorKind::TimeError(err))
    }
}

impl From<::ate::error::LoadError> for ContractError {
    fn from(err: ::ate::error::LoadError) -> Self {
        ContractErrorKind::CoreError(CoreErrorKind::LoadError(err.0)).into()
    }
}

impl From<::ate::error::LoadErrorKind> for ContractErrorKind {
    fn from(err: ::ate::error::LoadErrorKind) -> Self {
        ContractErrorKind::CoreError(CoreErrorKind::LoadError(err))
    }
}

impl From<::ate::error::CommitError> for ContractError {
    fn from(err: ::ate::error::CommitError) -> Self {
        ContractErrorKind::CoreError(CoreErrorKind::CommitError(err.0)).into()
    }
}

impl From<::ate::error::CommitErrorKind> for ContractErrorKind {
    fn from(err: ::ate::error::CommitErrorKind) -> Self {
        ContractErrorKind::CoreError(CoreErrorKind::CommitError(err))
    }
}

impl From<::ate::error::LockError> for ContractError {
    fn from(err: ::ate::error::LockError) -> Self {
        ContractErrorKind::CoreError(CoreErrorKind::LockError(err.0)).into()
    }
}

impl From<::ate::error::LockErrorKind> for ContractErrorKind {
    fn from(err: ::ate::error::LockErrorKind) -> Self {
        ContractErrorKind::CoreError(CoreErrorKind::LockError(err))
    }
}

impl From<ContractCreateFailed> for ContractError {
    fn from(err: ContractCreateFailed) -> ContractError {
        match err {
            ContractCreateFailed::AccountSuspended => {
                ContractErrorKind::CoreError(CoreErrorKind::AccountSuspended).into()
            }
            ContractCreateFailed::AlreadyExists(msg) => {
                ContractErrorKind::AlreadyExists(msg).into()
            }
            ContractCreateFailed::AuthenticationFailed => {
                ContractErrorKind::CoreError(CoreErrorKind::AuthenticationFailed).into()
            }
            ContractCreateFailed::Forbidden => {
                ContractErrorKind::CoreError(CoreErrorKind::Forbidden).into()
            }
            ContractCreateFailed::InvalidService => ContractErrorKind::InvalidService.into(),
            ContractCreateFailed::NoMasterKey => {
                ContractErrorKind::CoreError(CoreErrorKind::NoMasterKey).into()
            }
            ContractCreateFailed::OperatorBanned => {
                ContractErrorKind::CoreError(CoreErrorKind::OperatorBanned).into()
            }
            ContractCreateFailed::OperatorNotFound => {
                ContractErrorKind::CoreError(CoreErrorKind::OperatorNotFound).into()
            }
            ContractCreateFailed::UnsupportedCurrency(currency) => {
                ContractErrorKind::UnsupportedCurrency(currency).into()
            }
            ContractCreateFailed::InternalError(code) => {
                ContractErrorKind::CoreError(CoreErrorKind::InternalError(code)).into()
            }
        }
    }
}

impl From<ContractActionFailed> for ContractError {
    fn from(err: ContractActionFailed) -> ContractError {
        match err {
            ContractActionFailed::AccountSuspended => {
                ContractErrorKind::CoreError(CoreErrorKind::AccountSuspended).into()
            }
            ContractActionFailed::AuthenticationFailed => {
                ContractErrorKind::CoreError(CoreErrorKind::AuthenticationFailed).into()
            }
            ContractActionFailed::OperatorBanned => {
                ContractErrorKind::CoreError(CoreErrorKind::OperatorBanned).into()
            }
            ContractActionFailed::OperatorNotFound => {
                ContractErrorKind::CoreError(CoreErrorKind::OperatorNotFound).into()
            }
            ContractActionFailed::NoMasterKey => {
                ContractErrorKind::CoreError(CoreErrorKind::NoMasterKey).into()
            }
            ContractActionFailed::Forbidden => {
                ContractErrorKind::CoreError(CoreErrorKind::Forbidden).into()
            }
            ContractActionFailed::InvalidContractReference(reference) => {
                ContractErrorKind::InvalidReference(reference).into()
            }
            ContractActionFailed::InternalError(code) => {
                ContractErrorKind::CoreError(CoreErrorKind::InternalError(code)).into()
            }
        }
    }
}
