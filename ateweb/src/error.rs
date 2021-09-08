use error_chain::error_chain;
use ate::error::*;
use ate_files::error::FileSystemError;
use ate_files::error::FileSystemErrorKind;
use hyper::StatusCode;

error_chain! {
    types {
        WebServerError, WebServerErrorKind, ResultExt, Result;
    }
    links {
        LoadError(LoadError, LoadErrorKind);
        SerializationError(SerializationError, SerializationErrorKind);
        ChainCreationError(ChainCreationError, ChainCreationErrorKind);
        LockError(LockError, LockErrorKind);
        TransformError(TransformError, TransformErrorKind);
        FileSystemError(FileSystemError, FileSystemErrorKind);
    }
    foreign_links {
        HeaderStrError(http::header::ToStrError);
        HeaderValueError(http::header::InvalidHeaderValue);
        TungsteniteError(tungstenite::error::ProtocolError);
        HyperTungsteniteError(hyper_tungstenite::tungstenite::error::ProtocolError);
    }
    errors {
        BadHost(host: String) {
            description("Bad Host"),
            display("Bad Host - {}", host),
        }
        BadConfiguration(err: String) {
            description("Bad Configuration"),
            display("Bad Configuration - {}", err),
        }
        BadRequest(err: String) {
            description("Bad Request"),
            display("Bad Request - {}", err),
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
            WebServerError(WebServerErrorKind::BadRequest(_), _) => StatusCode::BAD_REQUEST,
            WebServerError(WebServerErrorKind::UnknownHost, _) => StatusCode::BAD_REQUEST,
            WebServerError(WebServerErrorKind::FileSystemError(FileSystemErrorKind::DoesNotExist), _) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR
        }
    }

    pub fn response_body(&self) -> String
    {
        let mut ret = match self {
            err => err.to_string()
        };
        if ret.ends_with("\n") == false {
            ret.push_str("\n");
        }
        ret
    }
}