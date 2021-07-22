#[allow(unused_imports)]
use log::{info, error, debug};

use serde::{Serialize, Deserialize};
use crate::header::*;
use crate::crypto::AteHash;

/// Unique key that represents this chain-of-trust. The design must
/// partition their data space into seperate chains to improve scalability
/// and performance as a single chain will reside on a single node within
/// the cluster.
#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChainKey {
    pub name: String,
    #[serde(skip)]
    pub hash: Option<AteHash>,
}

impl std::fmt::Display
for ChainKey {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl ChainKey
{
    pub fn new(mut val: String) -> ChainKey {
        while val.starts_with("/") == true {
            val = val[1..].to_string();
        }
        
        ChainKey {
            hash: Some(AteHash::from_bytes(val.as_bytes())),
            name: val,
        }
    }

    pub const ROOT: ChainKey = ChainKey {
        name: String::new(),
        hash: None,
    };

    pub fn with_name(&self, val: String) -> ChainKey
    {
        let mut ret = self.clone();
        ret.name = val;
        ret
    }

    pub fn with_temp_name(&self, val: String) -> ChainKey
    {
        let mut ret = self.clone();
        ret.name = format!("{}_{}", val, PrimaryKey::generate().as_hex_string());
        ret
    }

    pub fn hash(&self) -> AteHash
    {
        match &self.hash {
            Some(a) => a.clone(),
            None => AteHash::from_bytes(self.name.as_bytes())
        }
    }

    pub fn hash64(&self) -> u64
    {
        match &self.hash {
            Some(a) => a.to_u64(),
            None => AteHash::from_bytes(self.name.as_bytes()).to_u64()
        }
    }

    pub fn to_string(&self) -> String
    {
        self.name.clone()
    }
}

impl From<String>
for ChainKey
{
    fn from(val: String) -> ChainKey {
        ChainKey::new(val)
    }
}

impl From<&'static str>
for ChainKey
{
    fn from(val: &'static str) -> ChainKey {
        ChainKey::new(val.to_string())
    }
}

impl From<u64>
for ChainKey
{
    fn from(val: u64) -> ChainKey {
        ChainKey::new(val.to_string())
    }
}