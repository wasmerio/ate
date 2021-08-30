use error_chain::error_chain;

use ::ate::prelude::*;
use crate::request::*;

error_chain! {
    types {
        QueryError, QueryErrorKind, ResultExt, Result;
    }
    links {
        AteError(::ate::error::AteError, ::ate::error::AteErrorKind);
        ChainCreationError(::ate::error::ChainCreationError, ::ate::error::ChainCreationErrorKind);
        SerializationError(::ate::error::SerializationError, ::ate::error::SerializationErrorKind);
        InvokeError(::ate::error::InvokeError, ::ate::error::InvokeErrorKind);
    }
    foreign_links {
        IO(tokio::io::Error);
    }
    errors {
        NotFound {
            description("query failed as the user could not be found")
            display("query failed as the user could not be found")
        }
        Banned {
            description("query failed as the user has been banned")
            display("query failed as the user has been banned")
        }
        Suspended {
            description("query failed as the user has been suspended")
            display("query failed as the user has been suspended")
        }
        InternalError(code: u16) {
            description("query failed as the server experienced an internal error")
            display("query failed as the server experienced an internal error - code={}", code)
        }
    }
}

impl From<QueryError>
for AteError
{
    fn from(err: QueryError) -> AteError {
        AteErrorKind::ServiceError(err.to_string()).into()
    }
}

impl From<QueryFailed>
for QueryError {
    fn from(err: QueryFailed) -> QueryError {
        match err {
            QueryFailed::Banned => QueryErrorKind::Banned.into(),
            QueryFailed::NotFound => QueryErrorKind::NotFound.into(),
            QueryFailed::Suspended => QueryErrorKind::Suspended.into(),
            QueryFailed::InternalError(code) => QueryErrorKind::InternalError(code).into(),
        }
    }
}