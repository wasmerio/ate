#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]
#![allow(unused_imports)]
use tracing::{error, info, debug};

mod log;
mod test;

pub use super::utils::test::*;
pub use log::log_init;
pub use log::obscure_error;
pub use log::obscure_error_str;