use serde::{Serialize, Deserialize};

#[allow(unused_imports)]
use fastrand::u64;
use bytes::Bytes;
use std::{hash::{Hash}, mem::size_of};

use super::redo::LogFilePointer;

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct PrimaryKey
{
    key: u64,
}

impl Default for PrimaryKey
{
    fn default() -> PrimaryKey {
        PrimaryKey::generate()
    }
}

impl PrimaryKey {
    #[allow(dead_code)]
    pub fn generate() -> PrimaryKey {
        PrimaryKey {
            key: fastrand::u64(..),
        }
    }

    #[allow(dead_code)]
    pub fn new(key: u64) -> PrimaryKey {
        PrimaryKey {
            key: key
        }
    }

    #[allow(dead_code)]
    pub fn sizeof() -> u64 {
        size_of::<u64>() as u64
    }

    pub fn as_hex_string(&self) -> String {
        format!("{:X?}", self.key).to_string()
    }
}

#[derive(Debug, Clone)]
pub struct EventRaw
{
    pub meta: Bytes,
    pub data_hash: Option<super::crypto::Hash>,
    pub pointer: LogFilePointer,
}