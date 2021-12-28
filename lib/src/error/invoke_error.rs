use error_chain::error_chain;

error_chain! {
    types {
        InvokeError, InvokeErrorKind, ResultExt, Result;
    }
    links {
        LoadError(super::LoadError, super::LoadErrorKind);
        SerializationError(super::SerializationError, super::SerializationErrorKind);
        CommitError(super::CommitError, super::CommitErrorKind);
        TransformError(super::TransformError, super::TransformErrorKind);
        LockError(super::LockError, super::LockErrorKind);
    }
    foreign_links {
        IO(std::io::Error);
    }
    errors {
        PipeError(err: String) {
            description("command failed due to pipe error"),
            display("command failed due to pipe error - {}", err)
        }
        ServiceError(err: String) {
            description("command failed due to an error at the service"),
            display("command failed due to an error at the service - {}", err)
        }
        Timeout {
            description("command failed due to a timeout"),
            display("command failed due to a timeout")
        }
        Aborted {
            description("command failed as it was aborted"),
            display("command failed as it was aborted")
        }
        NoData {
            description("command failed as there was no data"),
            display("command failed as there was no data")
        }
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for InvokeError {
    fn from(err: tokio::sync::mpsc::error::SendError<T>) -> InvokeError {
        InvokeErrorKind::PipeError(err.to_string()).into()
    }
}

impl From<tokio::time::error::Elapsed> for InvokeError {
    fn from(_elapsed: tokio::time::error::Elapsed) -> InvokeError {
        InvokeErrorKind::Timeout.into()
    }
}

#[cfg(target_arch = "wasm32")]
impl From<wasm_bus_time::prelude::Elapsed> for InvokeError {
    fn from(_elapsed: wasm_bus_time::prelude::Elapsed) -> InvokeError {
        InvokeErrorKind::Timeout.into()
    }
}
