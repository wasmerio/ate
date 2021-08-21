#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]
#![allow(unused_imports)]
use tracing::{error, info, debug};

mod log;
mod test;
mod b64;

pub use super::utils::test::*;
pub use log::log_init;
pub use log::obscure_error;
pub use log::obscure_error_str;
pub use b64::vec_serialize;
pub use b64::vec_deserialize;
pub use b64::b16_serialize;
pub use b64::b16_deserialize;
pub use b64::b24_serialize;
pub use b64::b24_deserialize;
pub use b64::b32_serialize;
pub use b64::b32_deserialize;