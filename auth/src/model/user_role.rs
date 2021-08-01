#[allow(unused_imports)]
use tracing::{info, debug, warn, error, trace};
use serde::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum UserRole {
    Human,
    Robot,
}