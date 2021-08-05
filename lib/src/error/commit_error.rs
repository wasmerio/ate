use error_chain::error_chain;

error_chain! {
    types {
        CommitError, CommitErrorKind, ResultExt, Result;
    }
    links {
        CommsError(super::CommsError, super::CommsErrorKind);
        ValidationError(super::ValidationError, super::ValidationErrorKind);
        TransformError(super::TransformError, super::TransformErrorKind);
        LockError(super::LockError, super::LockErrorKind);
        LintError(super::LintError, super::LintErrorKind);
        TimeError(super::TimeError, super::TimeErrorKind);
        SinkError(super::SinkError, super::SinkErrorKind);
        SerializationError(super::SerializationError, super::SerializationErrorKind);
    }
    foreign_links {
        IO(::tokio::io::Error);
    }
    errors {
        Aborted {
            display("the transaction aborted before it could be completed"),
        }
        NewRootsAreDisabled {
            display("new root objects are currently not allowed for this chain"),
        }
        PipeError(err: String) {
            description("failed to commit the data due to an error receiving the result in the interprocess pipe"),
            display("failed to commit the data due to an error receiving the result in the interprocess pipe - {}", err.to_string()),
        }
        RootError(err: String) {
            description("failed to commit the data due to an error at the root server while processing the events"),
            display("failed to commit the data due to an error at the root server while processing the events - {}", err.to_string()),
        }
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>>
for CommitError
{
    fn from(err: tokio::sync::mpsc::error::SendError<T>) -> CommitError {
        CommitErrorKind::PipeError(err.to_string()).into()
    }   
}

impl<T> From<tokio::sync::broadcast::error::SendError<T>>
for CommitError
{
    fn from(err: tokio::sync::broadcast::error::SendError<T>) -> CommitError {
        CommitErrorKind::PipeError(err.to_string()).into()
    }   
}