#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmailVerification {
    pub code: String,
}