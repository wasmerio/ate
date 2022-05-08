#[cfg(feature = "sys")]
#[cfg(not(target_arch = "wasm32"))]
mod sys;
#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(feature = "sys")]
#[cfg(not(target_arch = "wasm32"))]
pub use sys::*;
#[cfg(target_arch = "wasm32")]
pub use wasm::*;
