pub mod abi;
#[cfg(feature = "backend")]
pub mod backend;
#[cfg(not(feature = "backend"))]
mod backend;
pub mod engine;
pub mod prelude;
#[cfg(feature = "process")]
pub mod process;
#[cfg(feature = "reqwest")]
pub mod reqwest;
#[cfg(feature = "rt")]
pub mod rt;
#[cfg(feature = "rt")]
pub mod task;
#[cfg(feature = "time")]
pub mod time;
#[cfg(feature = "ws")]
pub mod ws;

#[cfg(feature = "tokio")]
pub(crate) const MAX_MPSC: usize = std::usize::MAX >> 3;
