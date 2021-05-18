#[allow(unused_imports)]
use log::{info, warn, debug, error};
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