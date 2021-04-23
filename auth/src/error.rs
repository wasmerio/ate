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

impl From<LoginError>
for AteError
{
    fn from(err: LoginError) -> AteError {
        AteError::ServiceError(err.to_string())
    }
}

#[derive(Debug)]
pub enum GatherError
{
    IO(tokio::io::Error),
    Timeout,
    NotFound(String),
    NoAccess,
    ServerError(String),
    AteError(AteError)
}

impl From<tokio::io::Error>
for GatherError
{
    fn from(err: tokio::io::Error) -> GatherError {
        GatherError::IO(err)
    }
}

impl From<AteError>
for GatherError
{
    fn from(err: AteError) -> GatherError {
        GatherError::AteError(err)
    }
}

impl From<ChainCreationError>
for GatherError
{
    fn from(err: ChainCreationError) -> GatherError {
        GatherError::AteError(AteError::ChainCreationError(err))
    }
}

impl From<SerializationError>
for GatherError
{
    fn from(err: SerializationError) -> GatherError {
        GatherError::AteError(AteError::SerializationError(err))
    }
}

impl<E> From<InvokeError<E>>
for GatherError
where E: std::fmt::Debug
{
    fn from(err: InvokeError<E>) -> GatherError {
        GatherError::AteError(AteError::InvokeError(err.to_string()))
    }
}

impl std::fmt::Display
for GatherError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            GatherError::Timeout => {
                write!(f, "Login failed due to a timeout")
            },
            GatherError::NoAccess => {
                write!(f, "Gather failed as the session has no access to this group")
            },
            GatherError::IO(err) => {
                write!(f, "Gather failed due to an IO error ({})", err)
            },
            GatherError::NotFound(group) => {
                write!(f, "Gather failed as the group does not exist ({})", group)
            },
            GatherError::AteError(err) => {
                write!(f, "Gather failed ({})", err.to_string())
            },
            GatherError::ServerError(err) => {
                write!(f, "Gather failed due to an error on the server({})", err)
            },
        }
    }
}

impl From<GatherError>
for AteError
{
    fn from(err: GatherError) -> AteError {
        AteError::ServiceError(err.to_string())
    }
}

#[derive(Debug)]
pub enum CreateError
{
    IO(tokio::io::Error),
    AteError(AteError),
    MissingReadKey,
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
            CreateError::MissingReadKey => {
                write!(f, "Create failed as the session is missing a read key")
            },
        }
    }
}

impl From<CreateError>
for AteError
{
    fn from(err: CreateError) -> AteError {
        AteError::ServiceError(err.to_string())
    }
}

#[derive(Debug)]
pub enum GroupAddError
{
    IO(tokio::io::Error),
    AteError(AteError),
    InvalidPurpose(String),
    QueryError(QueryError),
    NoAccess,
    NoMasterKey
}

impl From<tokio::io::Error>
for GroupAddError
{
    fn from(err: tokio::io::Error) -> GroupAddError {
        GroupAddError::IO(err)
    }
}

impl From<ChainCreationError>
for GroupAddError
{
    fn from(err: ChainCreationError) -> GroupAddError {
        GroupAddError::AteError(AteError::ChainCreationError(err))
    }
}

impl From<SerializationError>
for GroupAddError
{
    fn from(err: SerializationError) -> GroupAddError {
        GroupAddError::AteError(AteError::SerializationError(err))
    }
}

impl From<AteError>
for GroupAddError
{
    fn from(err: AteError) -> GroupAddError {
        GroupAddError::AteError(err)
    }
}

impl From<QueryError>
for GroupAddError
{
    fn from(err: QueryError) -> GroupAddError {
        GroupAddError::QueryError(err)
    }
}

impl<E> From<InvokeError<E>>
for GroupAddError
where E: std::fmt::Debug
{
    fn from(err: InvokeError<E>) -> GroupAddError {
        GroupAddError::AteError(AteError::InvokeError(err.to_string()))
    }
}

impl std::fmt::Display
for GroupAddError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            GroupAddError::AteError(err) => {
                write!(f, "Group add failed ({})", err.to_string())
            },
            GroupAddError::QueryError(err) => {
                write!(f, "Group add failed while performing a query for the user ({})", err.to_string())
            },
            GroupAddError::IO(err) => {
                write!(f, "Group add failed due to an IO error ({})", err)
            },
            GroupAddError::NoMasterKey => {
                write!(f, "Group add failed as the server has not been properly initialized")
            },
            GroupAddError::InvalidPurpose(err) => {
                write!(f, "Group add failed as the role purpose was invalid - {}", err)
            },
            GroupAddError::NoAccess => {
                write!(f, "Group add failed as the referrer has no access to this group")
            },
        }
    }
}

impl From<GroupAddError>
for AteError
{
    fn from(err: GroupAddError) -> AteError {
        AteError::ServiceError(err.to_string())
    }
}

#[derive(Debug)]
pub enum QueryError
{
    IO(tokio::io::Error),
    AteError(AteError),
    NotFound,
    Banned,
    Suspended,
}

impl From<tokio::io::Error>
for QueryError
{
    fn from(err: tokio::io::Error) -> QueryError {
        QueryError::IO(err)
    }
}

impl From<ChainCreationError>
for QueryError
{
    fn from(err: ChainCreationError) -> QueryError {
        QueryError::AteError(AteError::ChainCreationError(err))
    }
}

impl From<SerializationError>
for QueryError
{
    fn from(err: SerializationError) -> QueryError {
        QueryError::AteError(AteError::SerializationError(err))
    }
}

impl From<AteError>
for QueryError
{
    fn from(err: AteError) -> QueryError {
        QueryError::AteError(err)
    }
}

impl<E> From<InvokeError<E>>
for QueryError
where E: std::fmt::Debug
{
    fn from(err: InvokeError<E>) -> QueryError {
        QueryError::AteError(AteError::InvokeError(err.to_string()))
    }
}

impl std::fmt::Display
for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            QueryError::AteError(err) => {
                write!(f, "Create failed ({})", err.to_string())
            },
            QueryError::IO(err) => {
                write!(f, "Create failed due to an IO error ({})", err)
            },
            QueryError::NotFound => {
                write!(f, "Create failed as the user could not be found")
            },
            QueryError::Banned => {
                write!(f, "Create failed as the user has been banned")
            },
            QueryError::Suspended => {
                write!(f, "Create failed as the user has been suspended")
            },
        }
    }
}

impl From<QueryError>
for AteError
{
    fn from(err: QueryError) -> AteError {
        AteError::ServiceError(err.to_string())
    }
}