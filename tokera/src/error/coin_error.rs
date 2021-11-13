use error_chain::error_chain;

use crate::request::*;

use super::*;

error_chain! {
    types {
        CoinError, CoinErrorKind, ResultExt, Result;
    }
    links {
        CoreError(super::CoreError, super::CoreErrorKind);
    }
    foreign_links {
        IO(tokio::io::Error);
    }
    errors {
        InvalidReference(reference_number: String) {
            description("invalid reference number"),
            display("invalid reference number ({})", reference_number),
        }
        EmailError(err: String) {
            description("failed to send email"),
            display("failed to send email - {}", err),
        }
        InvalidCurrencyError(currency: String) {
            description("invalid currency"),
            display("invalid currency ({})", currency),
        }
        InvalidCommodity {
            description("tHe supplied commodity is not vaild"),
            display("the supplied commodity is not vaild"),
        }
        InvalidCoin {
            description("the supplied coin is not valid"),
            display("the supplied coin is not valid")
        }
        NoOwnership {
            description("the coins you are accessing are not owned by you anymore"),
            display("the coins you are accessing are not owned by you anymore"),
        }
        InvalidAmount {
            description("the coin is not big enough to be carved by this amount of the carvng amount is invalid"),
            display("the coin is not big enough to be carved by this amount of the carvng amount is invalid"),
        }
    }
}

impl From<::ate::error::AteError>
for CoinError
{
    fn from(err: ::ate::error::AteError) -> Self {
        CoinErrorKind::CoreError(CoreErrorKind::AteError(err.0)).into()
    }
}

impl From<::ate::error::AteErrorKind>
for CoinErrorKind
{
    fn from(err: ::ate::error::AteErrorKind) -> Self {
        CoinErrorKind::CoreError(CoreErrorKind::AteError(err))
    }
}

impl From<::ate::error::ChainCreationError>
for CoinError
{
    fn from(err: ::ate::error::ChainCreationError) -> Self {
        CoinErrorKind::CoreError(CoreErrorKind::ChainCreationError(err.0)).into()
    }
}

impl From<::ate::error::ChainCreationErrorKind>
for CoinErrorKind
{
    fn from(err: ::ate::error::ChainCreationErrorKind) -> Self {
        CoinErrorKind::CoreError(CoreErrorKind::ChainCreationError(err))
    }
}

impl From<::ate::error::SerializationError>
for CoinError
{
    fn from(err: ::ate::error::SerializationError) -> Self {
        CoinErrorKind::CoreError(CoreErrorKind::SerializationError(err.0)).into()
    }
}

impl From<::ate::error::SerializationErrorKind>
for CoinError
{
    fn from(err: ::ate::error::SerializationErrorKind) -> Self {
        CoinErrorKind::CoreError(CoreErrorKind::SerializationError(err)).into()
    }
}

impl From<::ate::error::InvokeError>
for CoinError
{
    fn from(err: ::ate::error::InvokeError) -> Self {
        CoinErrorKind::CoreError(CoreErrorKind::InvokeError(err.0)).into()
    }
}

impl From<::ate::error::InvokeErrorKind>
for CoinError
{
    fn from(err: ::ate::error::InvokeErrorKind) -> Self {
        CoinErrorKind::CoreError(CoreErrorKind::InvokeError(err)).into()
    }
}

impl From<::ate::error::TimeError>
for CoinError
{
    fn from(err: ::ate::error::TimeError) -> Self {
        CoinErrorKind::CoreError(CoreErrorKind::TimeError(err.0)).into()
    }
}

impl From<::ate::error::TimeErrorKind>
for CoinErrorKind
{
    fn from(err: ::ate::error::TimeErrorKind) -> Self {
        CoinErrorKind::CoreError(CoreErrorKind::TimeError(err))
    }
}

impl From<::ate::error::LoadError>
for CoinError
{
    fn from(err: ::ate::error::LoadError) -> Self {
        CoinErrorKind::CoreError(CoreErrorKind::LoadError(err.0)).into()
    }
}

impl From<::ate::error::LoadErrorKind>
for CoinErrorKind
{
    fn from(err: ::ate::error::LoadErrorKind) -> Self {
        CoinErrorKind::CoreError(CoreErrorKind::LoadError(err))
    }
}

impl From<::ate::error::CommitError>
for CoinError
{
    fn from(err: ::ate::error::CommitError) -> Self {
        CoinErrorKind::CoreError(CoreErrorKind::CommitError(err.0)).into()
    }
}

impl From<::ate::error::CommitErrorKind>
for CoinErrorKind
{
    fn from(err: ::ate::error::CommitErrorKind) -> Self {
        CoinErrorKind::CoreError(CoreErrorKind::CommitError(err))
    }
}

impl From<::ate::error::LockError>
for CoinError
{
    fn from(err: ::ate::error::LockError) -> Self {
        CoinErrorKind::CoreError(CoreErrorKind::LockError(err.0)).into()
    }
}

impl From<::ate::error::LockErrorKind>
for CoinErrorKind
{
    fn from(err: ::ate::error::LockErrorKind) -> Self {
        CoinErrorKind::CoreError(CoreErrorKind::LockError(err))
    }
}

impl From<CoinCarveFailed>
for CoinError
{
    fn from(err: CoinCarveFailed) -> CoinError {
        match err {
            CoinCarveFailed::AuthenticationFailed => CoinErrorKind::CoreError(CoreErrorKind::AuthenticationFailed).into(),
            CoinCarveFailed::InvalidAmount => CoinErrorKind::InvalidAmount.into(),
            CoinCarveFailed::InvalidCommodity => CoinErrorKind::InvalidCommodity.into(),
            CoinCarveFailed::InvalidCoin => CoinErrorKind::InvalidCoin.into(),
            CoinCarveFailed::InternalError(code) => CoinErrorKind::CoreError(CoreErrorKind::InternalError(code)).into(),
        }
    }
}

impl From<CoinCollectFailed>
for CoinError
{
    fn from(err: CoinCollectFailed) -> CoinError {
        match err {
            CoinCollectFailed::AuthenticationFailed => CoinErrorKind::CoreError(CoreErrorKind::AuthenticationFailed).into(),
            CoinCollectFailed::InvalidCommodity => CoinErrorKind::InvalidCommodity.into(),
            CoinCollectFailed::InvalidCoin => CoinErrorKind::InvalidCoin.into(),
            CoinCollectFailed::OperatorBanned => CoinErrorKind::CoreError(CoreErrorKind::OperatorBanned).into(),
            CoinCollectFailed::InternalError(code) => CoinErrorKind::CoreError(CoreErrorKind::InternalError(code)).into(),
        }
    }
}

impl From<CoinRotateFailed>
for CoinError
{
    fn from(err: CoinRotateFailed) -> CoinError {
        match err {
            CoinRotateFailed::NoOwnership => CoinErrorKind::NoOwnership.into(),
            CoinRotateFailed::OperatorBanned => CoinErrorKind::CoreError(CoreErrorKind::OperatorBanned).into(),
            CoinRotateFailed::OperatorNotFound => CoinErrorKind::CoreError(CoreErrorKind::OperatorNotFound).into(),
            CoinRotateFailed::AuthenticationFailed => CoinErrorKind::CoreError(CoreErrorKind::AuthenticationFailed).into(),
            CoinRotateFailed::InvalidCommodity => CoinErrorKind::InvalidCommodity.into(),
            CoinRotateFailed::InvalidCoin => CoinErrorKind::InvalidCoin.into(),
            CoinRotateFailed::AccountSuspended => CoinErrorKind::CoreError(CoreErrorKind::AccountSuspended).into(),
            CoinRotateFailed::InternalError(code) => CoinErrorKind::CoreError(CoreErrorKind::InternalError(code)).into(),
        }
    }
}

impl From<CoinCombineFailed>
for CoinError
{
    fn from(err: CoinCombineFailed) -> CoinError {
        match err {
            CoinCombineFailed::AuthenticationFailed => CoinErrorKind::CoreError(CoreErrorKind::AuthenticationFailed).into(),
            CoinCombineFailed::OperatorBanned => CoinErrorKind::CoreError(CoreErrorKind::OperatorBanned).into(),
            CoinCombineFailed::InvalidCommodity => CoinErrorKind::InvalidCommodity.into(),
            CoinCombineFailed::InvalidCoin => CoinErrorKind::InvalidCoin.into(),
            CoinCombineFailed::InvalidRequest(err) => CoinErrorKind::CoreError(CoreErrorKind::InternalError(ate::utils::obscure_error_str(&err))).into(),
            CoinCombineFailed::InternalError(code) => CoinErrorKind::CoreError(CoreErrorKind::InternalError(code)).into(),
        }
    }
}