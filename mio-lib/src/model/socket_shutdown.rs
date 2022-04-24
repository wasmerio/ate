use std::fmt;
use serde::*;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum SocketShutdown
{
    Read,
    Write,
    Both,
}

impl fmt::Display
for SocketShutdown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use SocketShutdown::*;
        match self {
            Read => write!(f, "shutdown(read)"),
            Write => write!(f, "shutdown(write)"),
            Both => write!(f, "shutdown(both)"),
        }
    }
}

impl Into<std::net::Shutdown>
for SocketShutdown
{
    fn into(self) -> std::net::Shutdown {
        use SocketShutdown::*;
        match self {
            Read => std::net::Shutdown::Read,
            Write => std::net::Shutdown::Write,
            Both => std::net::Shutdown::Both,
        }
    }
}

impl From<std::net::Shutdown>
for SocketShutdown
{
    fn from(s: std::net::Shutdown) -> SocketShutdown {
        use SocketShutdown::*;
        match s {
            std::net::Shutdown::Read => Read,
            std::net::Shutdown::Write => Write,
            std::net::Shutdown::Both => Both,
        }
    }
}
