use serde::{Serialize, Deserialize};

#[allow(unused_imports)]
use fastrand::u64;
use std::{hash::{Hash}, mem::size_of};
#[allow(unused_imports)]
use super::meta::*;

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

impl Metadata
{
    pub fn for_data(key: PrimaryKey) -> Metadata {
        let mut ret = Metadata::default();
        ret.core.push(CoreMetadata::Data(key));
        return ret;
    }
    
    pub fn get_data_key(&self) -> Option<PrimaryKey> {
        self.core.iter().filter_map(
            |m| {
                match m
                {
                    CoreMetadata::Data(k) => Some(k.clone()),
                    CoreMetadata::Tombstone(k) => Some(k.clone()),
                     _ => None
                }
            }
        )
        .next()
    }

    #[allow(dead_code)]
    pub fn set_data_key(&mut self, key: PrimaryKey) {
        for core in self.core.iter_mut() {
            match core {
                CoreMetadata::Data(k) => {
                    if *k == key { return; }
                    *k = key;
                    return;
                },
                _ => {}
            }
        }
        self.core.push(CoreMetadata::Data(key));
    }
}