#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum UserStatus
{
    Nominal,
    Unverified,
    Locked(chrono::DateTime<chrono::Utc>),
}

impl Default
for UserStatus
{
    fn default() -> Self {
        UserStatus::Nominal
    }
}