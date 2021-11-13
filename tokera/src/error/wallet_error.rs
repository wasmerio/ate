use error_chain::error_chain;
use crate::request::*;
use super::*;

error_chain! {
    types {
        WalletError, WalletErrorKind, ResultExt, Result;
    }
    links {
        CoreError(super::CoreError, super::CoreErrorKind);
        CoinError(super::CoinError, super::CoinErrorKind);
        GatherError(super::GatherError, super::GatherErrorKind);
    }
    foreign_links {
        IO(tokio::io::Error);
    }
    errors {
        InvalidReference(reference_number: String) {
            description("invalid reference number"),
            display("invalid reference number ({})", reference_number),
        }
        WalletNotEmpty {
            description("wallet is not empty"),
            display("wallet is not empty"),
        }
        InvoiceAlreadyPaid(invoice_number: String) {
            description("invoice is already paid"),
            display("invoice is already paid ({})", invoice_number),
        }
        AlreadyPaid {
            description("the deposit has already been paid"),
            display("the deposit has already been paid"),
        }
        InsufficientCoins {
            description("insufficient coins"),
            display("insufficient coins"),
        }
        TooSmall {
            description("the withdrawl amount is too small"),
            display("the withdrawl amount is too small"),
        }
        NotDeposited {
            description("the funds do not exist as the deposit was never completed")
            display("the funds do not exist as the deposit was never completed")
        }
        WalletLocked {
            description("the wallet is currently locked for modification due to a concurrent operation"),
            display("the wallet is currently locked for modification due to a concurrent operation"),
        }
        EmailError(err: String) {
            description("failed to send email"),
            display("failed to send email - {}", err),
        }
    }
}

impl From<::ate::error::AteError>
for WalletError
{
    fn from(err: ::ate::error::AteError) -> Self {
        WalletErrorKind::CoreError(CoreErrorKind::AteError(err.0)).into()
    }
}

impl From<::ate::error::AteErrorKind>
for WalletErrorKind
{
    fn from(err: ::ate::error::AteErrorKind) -> Self {
        WalletErrorKind::CoreError(CoreErrorKind::AteError(err))
    }
}

impl From<::ate::error::ChainCreationError>
for WalletError
{
    fn from(err: ::ate::error::ChainCreationError) -> Self {
        WalletErrorKind::CoreError(CoreErrorKind::ChainCreationError(err.0)).into()
    }
}

impl From<::ate::error::ChainCreationErrorKind>
for WalletErrorKind
{
    fn from(err: ::ate::error::ChainCreationErrorKind) -> Self {
        WalletErrorKind::CoreError(CoreErrorKind::ChainCreationError(err))
    }
}

impl From<::ate::error::SerializationError>
for WalletError
{
    fn from(err: ::ate::error::SerializationError) -> Self {
        WalletErrorKind::CoreError(CoreErrorKind::SerializationError(err.0)).into()
    }
}

impl From<::ate::error::SerializationErrorKind>
for WalletErrorKind
{
    fn from(err: ::ate::error::SerializationErrorKind) -> Self {
        WalletErrorKind::CoreError(CoreErrorKind::SerializationError(err))
    }
}

impl From<::ate::error::InvokeError>
for WalletError
{
    fn from(err: ::ate::error::InvokeError) -> Self {
        WalletErrorKind::CoreError(CoreErrorKind::InvokeError(err.0)).into()
    }
}

impl From<::ate::error::InvokeErrorKind>
for WalletErrorKind
{
    fn from(err: ::ate::error::InvokeErrorKind) -> Self {
        WalletErrorKind::CoreError(CoreErrorKind::InvokeError(err))
    }
}

impl From<::ate::error::TimeError>
for WalletError
{
    fn from(err: ::ate::error::TimeError) -> Self {
        WalletErrorKind::CoreError(CoreErrorKind::TimeError(err.0)).into()
    }
}

impl From<::ate::error::TimeErrorKind>
for WalletErrorKind
{
    fn from(err: ::ate::error::TimeErrorKind) -> Self {
        WalletErrorKind::CoreError(CoreErrorKind::TimeError(err))
    }
}

impl From<::ate::error::LoadError>
for WalletError
{
    fn from(err: ::ate::error::LoadError) -> Self {
        WalletErrorKind::CoreError(CoreErrorKind::LoadError(err.0)).into()
    }
}

impl From<::ate::error::LoadErrorKind>
for WalletErrorKind
{
    fn from(err: ::ate::error::LoadErrorKind) -> Self {
        WalletErrorKind::CoreError(CoreErrorKind::LoadError(err))
    }
}

impl From<::ate::error::CommitError>
for WalletError
{
    fn from(err: ::ate::error::CommitError) -> Self {
        WalletErrorKind::CoreError(CoreErrorKind::CommitError(err.0)).into()
    }
}

impl From<::ate::error::CommitErrorKind>
for WalletErrorKind
{
    fn from(err: ::ate::error::CommitErrorKind) -> Self {
        WalletErrorKind::CoreError(CoreErrorKind::CommitError(err))
    }
}

impl From<::ate::error::LockError>
for WalletError
{
    fn from(err: ::ate::error::LockError) -> Self {
        WalletErrorKind::CoreError(CoreErrorKind::LockError(err.0)).into()
    }
}

impl From<::ate::error::LockErrorKind>
for WalletErrorKind
{
    fn from(err: ::ate::error::LockErrorKind) -> Self {
        WalletErrorKind::CoreError(CoreErrorKind::LockError(err))
    }
}

impl From<CancelDepositFailed>
for WalletError
{
    fn from(err: CancelDepositFailed) -> WalletError {
        match err {
            CancelDepositFailed::AuthenticationFailed => WalletErrorKind::CoreError(CoreErrorKind::AuthenticationFailed).into(),
            CancelDepositFailed::AlreadyPaid => WalletErrorKind::AlreadyPaid.into(),
            CancelDepositFailed::InvalidCommodity => WalletErrorKind::CoinError(CoinErrorKind::InvalidCommodity).into(),
            CancelDepositFailed::InvalidCoin => WalletErrorKind::CoinError(CoinErrorKind::InvalidCoin).into(),
            CancelDepositFailed::Forbidden => WalletErrorKind::CoreError(CoreErrorKind::Forbidden).into(),
            CancelDepositFailed::InternalError(code) => WalletErrorKind::CoreError(CoreErrorKind::InternalError(code)).into(),
        }
    }
}

impl From<DepositFailed>
for WalletError
{
    fn from(err: DepositFailed) -> WalletError {
        match err {
            DepositFailed::OperatorBanned => WalletErrorKind::CoreError(CoreErrorKind::OperatorBanned).into(),
            DepositFailed::OperatorNotFound => WalletErrorKind::CoreError(CoreErrorKind::OperatorNotFound).into(),
            DepositFailed::AuthenticationFailed => WalletErrorKind::CoreError(CoreErrorKind::AuthenticationFailed).into(),
            DepositFailed::AccountSuspended => WalletErrorKind::CoreError(CoreErrorKind::AccountSuspended).into(),
            DepositFailed::Forbidden => WalletErrorKind::CoreError(CoreErrorKind::Forbidden).into(),
            DepositFailed::UnsupportedCurrency(code) => WalletErrorKind::CoinError(CoinErrorKind::InvalidCurrencyError(code.to_string())).into(),
            DepositFailed::InternalError(code) => WalletErrorKind::CoreError(CoreErrorKind::InternalError(code)).into(),
        }
    }
}

impl From<WithdrawFailed>
for WalletError
{
    fn from(err: WithdrawFailed) -> WalletError {
        match err {
            WithdrawFailed::OperatorBanned => WalletErrorKind::CoreError(CoreErrorKind::OperatorBanned).into(),
            WithdrawFailed::OperatorNotFound => WalletErrorKind::CoreError(CoreErrorKind::OperatorNotFound).into(),
            WithdrawFailed::AuthenticationFailed => WalletErrorKind::CoreError(CoreErrorKind::AuthenticationFailed).into(),
            WithdrawFailed::AccountSuspended => WalletErrorKind::CoreError(CoreErrorKind::AccountSuspended).into(),
            WithdrawFailed::AlreadyWithdrawn => WalletErrorKind::AlreadyPaid.into(),
            WithdrawFailed::NotDeposited => WalletErrorKind::NotDeposited.into(),
            WithdrawFailed::TooSmall => WalletErrorKind::TooSmall.into(),
            WithdrawFailed::Forbidden => WalletErrorKind::CoreError(CoreErrorKind::Forbidden).into(),
            WithdrawFailed::InvalidCoin => WalletErrorKind::CoinError(CoinErrorKind::InvalidCoin).into(),
            WithdrawFailed::InvalidCommodity => WalletErrorKind::CoinError(CoinErrorKind::InvalidCommodity).into(),
            WithdrawFailed::UnsupportedCurrency(code) => WalletErrorKind::CoinError(CoinErrorKind::InvalidCurrencyError(code.to_string())).into(),
            WithdrawFailed::InternalError(code) => WalletErrorKind::CoreError(CoreErrorKind::InternalError(code)).into(),
        }
    }
}