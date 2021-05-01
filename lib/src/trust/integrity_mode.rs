#[allow(unused_imports)]
use log::{info, error, debug};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrityMode
{
    Centralized,
    Distributed
}

impl std::fmt::Display
for IntegrityMode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            IntegrityMode::Centralized => write!(f, "centralized"),
            IntegrityMode::Distributed => write!(f, "distributed")
        }
    }
}