#[cfg(feature = "sys")]
#[cfg(not(target_family = "wasm"))]
mod sys;
#[cfg(target_family = "wasm")]
mod wasm;

#[cfg(feature = "sys")]
#[cfg(not(target_family = "wasm"))]
pub use sys::*;
#[cfg(target_family = "wasm")]
pub use wasm::*;
