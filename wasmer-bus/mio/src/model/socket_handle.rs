use std::fmt;
use serde::*;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SocketHandle(pub i32);

impl fmt::Display
for SocketHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "handle({})", self.0)
    }
}