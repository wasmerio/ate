use error_chain::error_chain;
use ate::error::*;
use ate_files::error::FileSystemError;
use ate_files::error::FileSystemErrorKind;
use hyper::StatusCode;

error_chain! {
    types {
        WebServerError, WebServerErrorKind, WebServerResultExt, WebServerResult;
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

error_chain! {
    types {
        OrderError, OrderErrorKind, OrderResultExt, OrderResult;
    }
    links {
        SerializationError(SerializationError, SerializationErrorKind);
        CommitError(CommitError, CommitErrorKind);
        AcmeError(AcmeError, AcmeErrorKind);
    }
    errors {
        Pem(err: pem::PemError) {
            description("could not parse pem"),
            display("could not parse pem: {0}", err)
        }
        Rcgen(err: rcgen::RcgenError) {
            description("certificate generation error"),
            display("certificate generation error: {0}", err)
        }
        BadOrder(order: crate::acme::Order) {
            description("bad order object"),
            display("bad order object: {0:?}", order)
        }
        BadAuth(auth: crate::acme::Auth) {
            description("bad auth object"),
            display("bad auth object: {0:?}", auth)
        }
        TooManyAttemptsAuth(domain: String) {
            description("authorization failed too many times"),
            display("authorization for {0} failed too many times", domain)
        }
    }
}

impl From<pem::PemError>
for OrderError
{
    fn from(err: pem::PemError) -> OrderError {
        OrderErrorKind::Pem(err).into()
    }
}

impl From<rcgen::RcgenError>
for OrderError
{
    fn from(err: rcgen::RcgenError) -> OrderError {
        OrderErrorKind::Rcgen(err).into()
    }
}

error_chain! {
    types {
        SecurityError, SecurityErrorKind, SecurityResultExt, SecurityResult;
    }
    foreign_links {
        Json(serde_json::Error);
        Crypto(ring::error::Unspecified);
    }
}

error_chain! {
    types {
        AcmeError, AcmeErrorKind, AcmeResultExt, AcmeResult;
    }
    foreign_links {
        Io(std::io::Error);
        HyperError(hyper::Error);
        Rcgen(rcgen::RcgenError);
        Security(SecurityError);
        Json(serde_json::Error);
        KeyRejected(ring::error::KeyRejected);
        Crypto(ring::error::Unspecified);
        ToStrError(http::header::ToStrError);
    }
    errors {
        MissingHeader(name: &'static str) {
            description("missing header"),
            display("missing header ({})", name)
        }
        BadResponse(code: u16, response: String) {
            description("letsencrypt returned a bad response"),
            display("letsencrypt returned a bad response (code={}) - {}", code, response),
        }
        ApiError(err: super::ApiError) {
            description("letsencrypt returned an error"),
            display("letsencrypt returned an error (code={}) - {}", err.typ, err.detail),
        }
        NoTlsAlpn01Challenge {
            description("no tls alpn 01 challenge"),
            display("no tls alpn 01 challenge")
        }
    }
}

error_chain! {
    types {
        HttpError, HttpErrorKind, HttpResultExt, HttpResult;
    }
    foreign_links {
        IO(std::io::Error);
    }
    errors {
        UndefinedHost {
            description("could not determine host from url"),
            display("could not determine host from url")
        }
    }
}