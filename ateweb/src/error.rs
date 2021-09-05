use error_chain::error_chain;
use ate::error::*;
use ate_files::error::FileSystemError;
use ate_files::error::FileSystemErrorKind;
use hyper::StatusCode;

error_chain! {
    types {
        WebServerError, WebServerErrorKind, ResultExt, Result;
    }
    foreign_links {
        HeaderStrError(http::header::ToStrError);
        HeaderValueError(http::header::InvalidHeaderValue);
    }
    links {
        LoadError(LoadError, LoadErrorKind);
        SerializationError(SerializationError, SerializationErrorKind);
        ChainCreationError(ChainCreationError, ChainCreationErrorKind);
        LockError(LockError, LockErrorKind);
        TransformError(TransformError, TransformErrorKind);
        FileSystemError(FileSystemError, FileSystemErrorKind);
    }
    errors {
        BadHost(host: String) {
            description("Bad Host"),
            display("Bad Host - {}", host),
        }
        UnknownHost {
            description("Unknown Host"),
            display("Unknown Host"),
        }
    }
}

impl WebServerError
{
    pub fn status_code(&self) -> StatusCode
    {
        match self {
            WebServerError(WebServerErrorKind::BadHost(_), _) => StatusCode::BAD_GATEWAY,
            WebServerError(WebServerErrorKind::UnknownHost, _) => StatusCode::BAD_REQUEST,
            WebServerError(WebServerErrorKind::FileSystemError(FileSystemErrorKind::DoesNotExist), _) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}