use serde::*;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum UserStatus {
    Nominal,
    Unverified,
    Locked(chrono::DateTime<chrono::Utc>),
}

impl Default for UserStatus {
    fn default() -> Self {
        UserStatus::Nominal
    }
}
