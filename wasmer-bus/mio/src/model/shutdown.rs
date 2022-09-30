use serde::*;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum Shutdown
{
    Read,
    Write,
    Both,
}

impl Into<std::net::Shutdown>
for Shutdown
{
    fn into(self) -> std::net::Shutdown {
        use Shutdown::*;
        match self {
            Read => std::net::Shutdown::Read,
            Write => std::net::Shutdown::Write,
            Both => std::net::Shutdown::Both,
        }
    }
}

impl From<std::net::Shutdown>
for Shutdown
{
    fn from(s: std::net::Shutdown) -> Shutdown {
        use Shutdown::*;
        match s {
            std::net::Shutdown::Read => Read,
            std::net::Shutdown::Write => Write,
            std::net::Shutdown::Both => Both,
        }
    }
}
