use error_chain::error_chain;

error_chain! {
    types {
        AteError, AteErrorKind, ResultExt, Result;
    }
    links {
        BusError(super::BusError, super::BusErrorKind);
        ChainCreationError(super::ChainCreationError, super::ChainCreationErrorKind);
        CommitError(super::CommitError, super::CommitErrorKind);
        CommsError(super::CommsError, super::CommsErrorKind);
        CompactError(super::CompactError, super::CompactErrorKind);
        CryptoError(super::CryptoError, super::CryptoErrorKind);
        InvokeError(super::InvokeError, super::InvokeErrorKind);
        LintError(super::LintError, super::LintErrorKind);
        LoadError(super::LoadError, super::LoadErrorKind);
        LockError(super::LockError, super::LockErrorKind);
        SerializationError(super::SerializationError, super::SerializationErrorKind);
        SinkError(super::SinkError, super::SinkErrorKind);
        TimeError(super::TimeError, super::TimeErrorKind);
        TransformError(super::TransformError, super::TransformErrorKind);
        TrustError(super::TrustError, super::TrustErrorKind);
        ValidationError(super::ValidationError, super::ValidationErrorKind);
    }
    foreign_links {
        IO(::tokio::io::Error);
        UrlInvalid(::url::ParseError);
        ProcessError(super::process_error::ProcessError);
    }
    errors {
        NotImplemented {
            display("not implemented")
        }
    }
}

impl From<serde_json::Error>
for AteError
{
    fn from(err: serde_json::Error) -> AteError {
        AteErrorKind::SerializationError(super::SerializationErrorKind::SerdeError(err.to_string()).into()).into()
    } 
}

impl From<tokio::sync::watch::error::RecvError>
for AteError
{
    fn from(err: tokio::sync::watch::error::RecvError) -> AteError {
        AteErrorKind::IO(tokio::io::Error::new(tokio::io::ErrorKind::Other, err.to_string())).into()
    }   
}

impl<T> From<tokio::sync::watch::error::SendError<T>>
for AteError
where T: std::fmt::Debug
{
    fn from(err: tokio::sync::watch::error::SendError<T>) -> AteError {
        AteErrorKind::IO(tokio::io::Error::new(tokio::io::ErrorKind::Other, err.to_string())).into()
    }   
}