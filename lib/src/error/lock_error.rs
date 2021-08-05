use error_chain::error_chain;

error_chain! {
    types {
        LockError, LockErrorKind, ResultExt, Result;
    }
    links {
        SerializationError(super::SerializationError, super::SerializationErrorKind);
        LintError(super::LintError, super::LintErrorKind);
    }
    errors {
        CommitError(err: String) {
            description("failed to lock the data object due to issue committing the event to the pipe"),
            display("failed to lock the data object due to issue committing the event to the pipe - {}", err),
        }
        ReceiveError(err: String) {
            description("failed to lock the data object due to an error receiving on the pipe"),
            display("failed to lock the data object due to an error receiving on the pipe - {}", err),
        }
        WeakDio {
            display("the dIO that created this object has gone out of scope")
        }
    }
}

impl From<super::CommitError>
for LockError
{
    fn from(err: super::CommitError) -> LockError {
        LockErrorKind::CommitError(err.to_string()).into()
    }   
}

impl From<super::CommitErrorKind>
for LockError
{
    fn from(err: super::CommitErrorKind) -> LockError {
        LockErrorKind::CommitError(err.to_string()).into()
    }   
}