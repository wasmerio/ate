use ::ate::prelude::*;
use ::ate::error::*;

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

impl From<InvokeError>
for LoginError
{
    fn from(err: InvokeError) -> LoginError {
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

impl From<InvokeError>
for GatherError
{
    fn from(err: InvokeError) -> GatherError {
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
    PasswordMismatch,
    AlreadyExists,
    InvalidEmail,
    NoMoreRoom,
    InvalidName,
    NoMasterKey,
    QueryError(QueryError)
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

impl From<QueryError>
for CreateError
{
    fn from(err: QueryError) -> CreateError {
        CreateError::QueryError(err)
    }
}

impl From<InvokeError>
for CreateError
{
    fn from(err: InvokeError) -> CreateError {
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
            CreateError::QueryError(err) => {
                write!(f, "Create failed ({})", err.to_string())
            },
            CreateError::IO(err) => {
                write!(f, "Create failed due to an IO error ({})", err)
            },
            CreateError::AlreadyExists => {
                write!(f, "Create failed as the account or group already exists")
            },
            CreateError::InvalidEmail => {
                write!(f, "Create failed as the email address is invalid")
            },
            CreateError::NoMoreRoom => {
                write!(f, "Create failed as the account or group as there is no more room - try a different name")
            },
            CreateError::NoMasterKey => {
                write!(f, "Create failed as the server does not possess the master key")
            },
            CreateError::InvalidName => {
                write!(f, "Create failed as the account or group name is invalid")
            },
            CreateError::PasswordMismatch => {
                write!(f, "Create failed as the passwords did not match")
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
pub enum GroupUserAddError
{
    IO(tokio::io::Error),
    AteError(AteError),
    InvalidPurpose(String),
    QueryError(QueryError),
    NoAccess,
    NoMasterKey
}

impl From<tokio::io::Error>
for GroupUserAddError
{
    fn from(err: tokio::io::Error) -> GroupUserAddError {
        GroupUserAddError::IO(err)
    }
}

impl From<ChainCreationError>
for GroupUserAddError
{
    fn from(err: ChainCreationError) -> GroupUserAddError {
        GroupUserAddError::AteError(AteError::ChainCreationError(err))
    }
}

impl From<SerializationError>
for GroupUserAddError
{
    fn from(err: SerializationError) -> GroupUserAddError {
        GroupUserAddError::AteError(AteError::SerializationError(err))
    }
}

impl From<AteError>
for GroupUserAddError
{
    fn from(err: AteError) -> GroupUserAddError {
        GroupUserAddError::AteError(err)
    }
}

impl From<QueryError>
for GroupUserAddError
{
    fn from(err: QueryError) -> GroupUserAddError {
        GroupUserAddError::QueryError(err)
    }
}

impl From<InvokeError>
for GroupUserAddError
{
    fn from(err: InvokeError) -> GroupUserAddError {
        GroupUserAddError::AteError(AteError::InvokeError(err.to_string()))
    }
}

impl std::fmt::Display
for GroupUserAddError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            GroupUserAddError::AteError(err) => {
                write!(f, "Group user add failed ({})", err.to_string())
            },
            GroupUserAddError::QueryError(err) => {
                write!(f, "Group user add failed while performing a query for the user ({})", err.to_string())
            },
            GroupUserAddError::IO(err) => {
                write!(f, "Group user add failed due to an IO error ({})", err)
            },
            GroupUserAddError::NoMasterKey => {
                write!(f, "Group user add failed as the server has not been properly initialized")
            },
            GroupUserAddError::InvalidPurpose(err) => {
                write!(f, "Group user add failed as the role purpose was invalid - {}", err)
            },
            GroupUserAddError::NoAccess => {
                write!(f, "Group user add failed as the referrer has no access to this group")
            },
        }
    }
}

impl From<GroupUserAddError>
for AteError
{
    fn from(err: GroupUserAddError) -> AteError {
        AteError::ServiceError(err.to_string())
    }
}

#[derive(Debug)]
pub enum GroupDetailsError
{
    IO(tokio::io::Error),
    AteError(AteError),
    GroupNotFound,
    NoAccess,
}

impl From<tokio::io::Error>
for GroupDetailsError
{
    fn from(err: tokio::io::Error) -> GroupDetailsError {
        GroupDetailsError::IO(err)
    }
}

impl From<ChainCreationError>
for GroupDetailsError
{
    fn from(err: ChainCreationError) -> GroupDetailsError {
        GroupDetailsError::AteError(AteError::ChainCreationError(err))
    }
}

impl From<SerializationError>
for GroupDetailsError
{
    fn from(err: SerializationError) -> GroupDetailsError {
        GroupDetailsError::AteError(AteError::SerializationError(err))
    }
}

impl From<AteError>
for GroupDetailsError
{
    fn from(err: AteError) -> GroupDetailsError {
        GroupDetailsError::AteError(err)
    }
}

impl From<InvokeError>
for GroupDetailsError
{
    fn from(err: InvokeError) -> GroupDetailsError {
        GroupDetailsError::AteError(AteError::InvokeError(err.to_string()))
    }
}

impl std::fmt::Display
for GroupDetailsError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            GroupDetailsError::AteError(err) => {
                write!(f, "Group details failed ({})", err.to_string())
            },
            GroupDetailsError::IO(err) => {
                write!(f, "Group user add failed due to an IO error ({})", err)
            },
            GroupDetailsError::GroupNotFound => {
                write!(f, "Group details failed as the group does not exist")
            },
            GroupDetailsError::NoAccess => {
                write!(f, "Group details failed as the referrer has no access to this group")
            },
        }
    }
}

impl From<GroupDetailsError>
for AteError
{
    fn from(err: GroupDetailsError) -> AteError {
        AteError::ServiceError(err.to_string())
    }
}

#[derive(Debug)]
pub enum GroupUserRemoveError
{
    IO(tokio::io::Error),
    AteError(AteError),
    InvalidPurpose(String),
    QueryError(QueryError),
    NoAccess,
    NoMasterKey,
    GroupNotFound,
    RoleNotFound,
    NothingToRemove,
}

impl From<tokio::io::Error>
for GroupUserRemoveError
{
    fn from(err: tokio::io::Error) -> GroupUserRemoveError {
        GroupUserRemoveError::IO(err)
    }
}

impl From<ChainCreationError>
for GroupUserRemoveError
{
    fn from(err: ChainCreationError) -> GroupUserRemoveError {
        GroupUserRemoveError::AteError(AteError::ChainCreationError(err))
    }
}

impl From<SerializationError>
for GroupUserRemoveError
{
    fn from(err: SerializationError) -> GroupUserRemoveError {
        GroupUserRemoveError::AteError(AteError::SerializationError(err))
    }
}

impl From<AteError>
for GroupUserRemoveError
{
    fn from(err: AteError) -> GroupUserRemoveError {
        GroupUserRemoveError::AteError(err)
    }
}

impl From<QueryError>
for GroupUserRemoveError
{
    fn from(err: QueryError) -> GroupUserRemoveError {
        GroupUserRemoveError::QueryError(err)
    }
}

impl From<InvokeError>
for GroupUserRemoveError
{
    fn from(err: InvokeError) -> GroupUserRemoveError {
        GroupUserRemoveError::AteError(AteError::InvokeError(err.to_string()))
    }
}

impl std::fmt::Display
for GroupUserRemoveError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            GroupUserRemoveError::AteError(err) => {
                write!(f, "Group user remove failed ({})", err.to_string())
            },
            GroupUserRemoveError::QueryError(err) => {
                write!(f, "Group user remove failed while performing a query for the user ({})", err.to_string())
            },
            GroupUserRemoveError::IO(err) => {
                write!(f, "Group user remove failed due to an IO error ({})", err)
            },
            GroupUserRemoveError::NoMasterKey => {
                write!(f, "Group user remove failed as the server has not been properly initialized")
            },
            GroupUserRemoveError::InvalidPurpose(err) => {
                write!(f, "Group user remove failed as the role purpose was invalid - {}", err)
            },
            GroupUserRemoveError::NoAccess => {
                write!(f, "Group user remove failed as the referrer has no access to this group")
            },
            GroupUserRemoveError::GroupNotFound => {
                write!(f, "Group user remove failed as the group does not exist")
            },
            GroupUserRemoveError::RoleNotFound => {
                write!(f, "Group user remove failed as the group role does not exist")
            },
            GroupUserRemoveError::NothingToRemove => {
                write!(f, "Group user remove failed as the user is not a member of this group role")
            },
        }
    }
}

impl From<GroupUserRemoveError>
for AteError
{
    fn from(err: GroupUserRemoveError) -> AteError {
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

impl From<InvokeError>
for QueryError
{
    fn from(err: InvokeError) -> QueryError {
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