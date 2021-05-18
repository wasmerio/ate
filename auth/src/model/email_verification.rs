#[allow(unused_imports)]
use log::{info, warn, debug, error};
use serde::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmailVerification {
    pub code: String,
}