use ate::error::AteError;
use ate_auth::error::*;

#[derive(Debug)]
pub enum CommandError
{
    LoginError(LoginError),
    CreateError(CreateError),
    AteError(AteError),
}

impl From<LoginError>
for CommandError
{
    fn from(err: LoginError) -> CommandError {
        CommandError::LoginError(err)
    }
}

impl From<CreateError>
for CommandError
{
    fn from(err: CreateError) -> CommandError {
        CommandError::CreateError(err)
    }
}

impl From<AteError>
for CommandError
{
    fn from(err: AteError) -> CommandError {
        CommandError::AteError(err)
    }
}

impl std::fmt::Display
for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CommandError::LoginError(err) => {
                write!(f, "{}", err)
            },
            CommandError::CreateError(err) => {
                write!(f, "{}", err)
            },
            CommandError::AteError(err) => {
                write!(f, "{}", err)
            },
        }
    }
}