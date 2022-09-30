use std::fmt::{Display, Formatter, self};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum SocketState {
    Opening,
    Opened,
    Closed,
    Failed,
}

impl Display for SocketState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SocketState::Opening => write!(f, "opening"),
            SocketState::Opened => write!(f, "opened"),
            SocketState::Closed => write!(f, "closed"),
            SocketState::Failed => write!(f, "failed"),
        }
    }
}
