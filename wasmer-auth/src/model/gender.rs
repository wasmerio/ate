use serde::*;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Gender {
    Unspecified,
    Male,
    Female,
    Other,
}

impl Default for Gender {
    fn default() -> Self {
        Gender::Unspecified
    }
}
