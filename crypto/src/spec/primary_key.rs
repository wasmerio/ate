use std::mem::size_of;
use std::cell::RefCell;
use serde::*;

use crate::AteHash;

/// All event and data objects within ATE have a primary key that uniquely represents
/// it and allows it to be indexed and referenced. This primary key can be derived from
/// other input data like strings or numbers in order to make object lookups that are
/// static (e.g. root nodes)
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct PrimaryKey {
    key: u64,
}

impl Default for PrimaryKey {
    fn default() -> PrimaryKey {
        PrimaryKey::generate()
    }
}

impl PrimaryKey {
    thread_local! {
        static CURRENT: RefCell<Option<PrimaryKey>> = RefCell::new(None)
    }

    pub fn current_get() -> Option<PrimaryKey> {
        PrimaryKey::CURRENT.with(|key| {
            let key = key.borrow();
            return key.clone();
        })
    }

    pub fn current_set(val: Option<PrimaryKey>) -> Option<PrimaryKey> {
        PrimaryKey::CURRENT.with(|key| {
            let mut key = key.borrow_mut();
            match val {
                Some(a) => key.replace(a),
                None => key.take(),
            }
        })
    }

    #[allow(dead_code)]
    pub fn generate() -> PrimaryKey {
        PrimaryKey {
            key: fastrand::u64(..),
        }
    }

    #[allow(dead_code)]
    pub fn new(key: u64) -> PrimaryKey {
        PrimaryKey { key: key }
    }

    #[allow(dead_code)]
    pub fn sizeof() -> u64 {
        size_of::<u64>() as u64
    }

    pub fn as_hex_string(&self) -> String {
        format!("{:X?}", self.key).to_string()
    }

    pub fn as_fixed_hex_string(&self) -> String {
        let hex = format!("{:016X?}", self.key).to_string();
        let hex = hex.to_lowercase();
        format!("{}", &hex[..16])
    }

    pub fn as_u64(&self) -> u64 {
        self.key
    }

    pub fn from_ext(val: AteHash, min: u64, max: u64) -> PrimaryKey {
        let v = val.val;
        let bytes = [v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]];

        let range = max - min;
        let key = (u64::from_be_bytes(bytes) % range) + min;
        PrimaryKey { key }
    }
}

impl From<AteHash> for PrimaryKey {
    fn from(val: AteHash) -> PrimaryKey {
        let v = val.val;
        let bytes = [v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]];
        PrimaryKey {
            key: u64::from_be_bytes(bytes),
        }
    }
}

impl From<String> for PrimaryKey {
    fn from(val: String) -> PrimaryKey {
        PrimaryKey::from(AteHash::from(val))
    }
}

impl From<&'static str> for PrimaryKey {
    fn from(val: &'static str) -> PrimaryKey {
        PrimaryKey::from(AteHash::from(val))
    }
}

impl From<u64> for PrimaryKey {
    fn from(val: u64) -> PrimaryKey {
        PrimaryKey { key: val }
    }
}

impl std::fmt::Display for PrimaryKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_hex_string())
    }
}

#[test]
fn test_key_hex() {
    let key = PrimaryKey::from(1 as u64);
    assert_eq!(key.as_fixed_hex_string(), "0000000000000001".to_string());
}
