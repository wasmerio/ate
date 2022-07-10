use serde::*;
use std::sync::Arc;
#[allow(unused_imports)]
use wasmer_bus::macros::*;

#[wasmer_bus(format = "json")]
pub trait Tok {
    async fn user_exists(
        &self,
        email: String,
    ) -> TokResult<bool>;

    async fn user_create(
        &self,
        email: String,
        password: String
    ) -> TokResult<()>;

    async fn login(
        &self,
        email: String,
        password: String,
        code: Option<String>
    ) -> Arc<dyn Session>;
}

#[wasmer_bus(format = "json")]
pub trait Session {
    async fn user_details(
        &self
    ) -> TokResult<()>;
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TokError {
    Unauthorized,
    InvalidUser,
    NotImplemented,
    InternalError(u16),
}

impl std::fmt::Display for TokError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokError::Unauthorized => write!(f, "unauthorized"),
            TokError::InvalidUser => write!(f, "invalid user"),
            TokError::NotImplemented => write!(f, "not implemented"),
            TokError::InternalError(code) => write!(f, "internal error ({})", code),
        }
    }
}

impl std::error::Error for TokError {
}

pub type TokResult<T> = Result<T, TokError>;