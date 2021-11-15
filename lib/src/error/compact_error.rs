use error_chain::error_chain;
use tokio::sync::broadcast;
use tokio::sync::watch;

error_chain! {
    types {
        CompactError, CompactErrorKind, ResultExt, Result;
    }
    links {
        SinkError(super::SinkError, super::SinkErrorKind);
        TimeError(super::TimeError, super::TimeErrorKind);
        SerializationError(super::SerializationError, super::SerializationErrorKind);
        LoadError(super::LoadError, super::LoadErrorKind);
    }
    foreign_links {
        IO(tokio::io::Error);
    }
    errors {
        WatchError(err: String) {
            description("failed to compact the chain due to an error in watch notification"),
            display("failed to compact the chain due to an error in watch notification - {}", err),
        }
        BroadcastError(err: String) {
            description("failed to compact the chain due to an error in broadcast notification"),
            display("failed to compact the chain due to an error in broadcast notification - {}", err)
        }
        Aborted {
            description("compacting has been aborted")
            display("compacting has been aborted")
        }
    }
}

impl From<watch::error::RecvError> for CompactError {
    fn from(err: watch::error::RecvError) -> CompactError {
        CompactErrorKind::WatchError(err.to_string()).into()
    }
}

impl<T> From<watch::error::SendError<T>> for CompactError
where
    T: std::fmt::Debug,
{
    fn from(err: watch::error::SendError<T>) -> CompactError {
        CompactErrorKind::WatchError(err.to_string()).into()
    }
}

impl From<broadcast::error::RecvError> for CompactError {
    fn from(err: broadcast::error::RecvError) -> CompactError {
        CompactErrorKind::BroadcastError(err.to_string()).into()
    }
}

impl<T> From<broadcast::error::SendError<T>> for CompactError
where
    T: std::fmt::Debug,
{
    fn from(err: broadcast::error::SendError<T>) -> CompactError {
        CompactErrorKind::BroadcastError(err.to_string()).into()
    }
}
