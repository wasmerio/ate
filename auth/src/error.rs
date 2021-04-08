use ate::{error::{ChainCreationError, InvokeError}, prelude::AteError};

#[derive(Debug)]
pub enum LoginError
{
    IO(tokio::io::Error),
    Timeout,
    NotFound(String),
    AccountLocked,
    ServerError(String),
    AteError(AteError)
}

impl From<tokio::io::Error>
for LoginError
{
    fn from(err: tokio::io::Error) -> LoginError {
        LoginError::IO(err)
    }
}

impl From<AteError>
for LoginError
{
    fn from(err: AteError) -> LoginError {
        LoginError::AteError(err)
    }
}

impl From<ChainCreationError>
for LoginError
{
    fn from(err: ChainCreationError) -> LoginError {
        LoginError::AteError(AteError::ChainCreationError(err))
    }
}

impl<E> From<InvokeError<E>>
for LoginError
where E: std::fmt::Debug
{
    fn from(err: InvokeError<E>) -> LoginError {
        LoginError::AteError(AteError::InvokeError(err.to_string()))
    }
}

impl std::fmt::Display
for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LoginError::Timeout => {
                write!(f, "Login failed due to a timeout")
            },
            LoginError::AccountLocked => {
                write!(f, "Login failed as the account is locked")
            },
            LoginError::IO(err) => {
                write!(f, "Login failed due to an IO error ({})", err)
            },
            LoginError::NotFound(email) => {
                write!(f, "Login failed as the account does not exist ({})", email)
            },
            LoginError::AteError(err) => {
                write!(f, "Login failed ({})", err.to_string())
            },
            LoginError::ServerError(err) => {
                write!(f, "Login failed due to an error on the server({})", err)
            },
        }
    }
}