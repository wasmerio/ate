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