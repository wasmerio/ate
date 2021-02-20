extern crate uuid;

use serde::{Serialize, Deserialize};
use uuid::Uuid;
use std::hash::{Hash};

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash, Eq, PartialEq)]
pub struct Header
{
    pub key: String,
    pub version: Uuid,
}