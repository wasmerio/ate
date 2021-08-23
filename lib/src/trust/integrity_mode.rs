#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::{Serialize, Deserialize};
use crate::prelude::AteHash;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrityMode
{
    Centralized(AteHash),
    Distributed
}

impl IntegrityMode
{
    pub fn is_centralized(&self) -> bool
    {
        match self {
            IntegrityMode::Centralized(_) => true,
            _ => false
        }
    }
}

impl std::fmt::Display
for IntegrityMode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            IntegrityMode::Centralized(a) => write!(f, "centralized(session={})", a),
            IntegrityMode::Distributed => write!(f, "distributed")
        }
    }
}