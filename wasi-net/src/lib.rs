pub mod abi;
pub mod engine;
#[cfg(feature = "backend")]
pub mod backend;
#[cfg(not(feature = "backend"))]
mod backend;
pub mod prelude;
#[cfg(feature = "process")]
pub mod process;
#[cfg(feature = "reqwest")]
pub mod reqwest;
#[cfg(feature = "ws")]
pub mod ws;

pub(crate) const MAX_MPSC: usize = std::usize::MAX >> 3;