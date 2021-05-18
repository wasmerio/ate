#[allow(unused_imports)]
use log::{info, warn, debug, error};
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