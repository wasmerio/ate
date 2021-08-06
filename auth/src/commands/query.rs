#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::*;
use ate::prelude::*;

use crate::model::Advert;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryRequest
{
    pub identity: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryResponse
{
    pub advert: Advert,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum QueryFailed
{
    NotFound,
    Banned,
    Suspended,
    InternalError(u16),
}

impl<E> From<E>
for QueryFailed
where E: std::error::Error + Sized
{
    fn from(err: E) -> Self {
        QueryFailed::InternalError(ate::utils::obscure_error(err))
    }
}