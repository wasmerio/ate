use crate::prelude::*;

pub fn chain_key_16hex(val: &str, prefix: Option<&str>) -> ChainKey
{
    let hash = AteHash::from(val.to_string());
    let hex = hash.to_hex_string().to_lowercase();
    match prefix {
        Some(prefix) => ChainKey::new(format!("{}-{}", prefix, &hex[..16])),
        None => ChainKey::new(format!("{}", &hex[..16]))
    }
}

pub fn chain_key_4hex(val: &str, prefix: Option<&str>) -> ChainKey
{
    let hash = AteHash::from(val.to_string());
    let hex = hash.to_hex_string().to_lowercase();
    match prefix {
        Some(prefix) => ChainKey::new(format!("{}-{}", prefix, &hex[..4])),
        None => ChainKey::new(format!("{}", &hex[..4]))
    }
}