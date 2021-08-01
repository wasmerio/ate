#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Gender
{
    Unspecified,
    Male,
    Female,
    Other,
}

impl Default
for Gender
{
    fn default() -> Self {
        Gender::Unspecified
    }
}