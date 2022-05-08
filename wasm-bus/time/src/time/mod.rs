mod sleep;
mod timeout;

pub use sleep::*;
pub use timeout::*;

#[allow(dead_code)]
const WAPM_NAME: &'static str = "os";
