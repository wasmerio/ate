use ate::prelude::*;
use ate::error::*;

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

impl From<SerializationError>
for LoginError
{
    fn from(err: SerializationError) -> LoginError {
        LoginError::AteError(AteError::SerializationError(err))
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

#[derive(Debug)]
pub enum CreateError
{
    IO(tokio::io::Error),
    AteError(AteError),
    AlreadyExists
}

impl From<tokio::io::Error>
for CreateError
{
    fn from(err: tokio::io::Error) -> CreateError {
        CreateError::IO(err)
    }
}

impl From<ChainCreationError>
for CreateError
{
    fn from(err: ChainCreationError) -> CreateError {
        CreateError::AteError(AteError::ChainCreationError(err))
    }
}

impl From<SerializationError>
for CreateError
{
    fn from(err: SerializationError) -> CreateError {
        CreateError::AteError(AteError::SerializationError(err))
    }
}

impl From<AteError>
for CreateError
{
    fn from(err: AteError) -> CreateError {
        CreateError::AteError(err)
    }
}

impl<E> From<InvokeError<E>>
for CreateError
where E: std::fmt::Debug
{
    fn from(err: InvokeError<E>) -> CreateError {
        CreateError::AteError(AteError::InvokeError(err.to_string()))
    }
}

impl std::fmt::Display
for CreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CreateError::AteError(err) => {
                write!(f, "Create failed ({})", err.to_string())
            },
            CreateError::IO(err) => {
                write!(f, "Create failed due to an IO error ({})", err)
            },
            CreateError::AlreadyExists => {
                write!(f, "Create failed as the account already exists")
            },
        }
    }
}