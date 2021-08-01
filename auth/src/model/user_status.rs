#[allow(unused_imports)]
use tracing::{info, debug, warn, error, trace};
use serde::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum UserStatus
{
    Nominal,
    Unverified,
    Locked,
}

impl Default
for UserStatus
{
    fn default() -> Self {
        UserStatus::Nominal
    }
}