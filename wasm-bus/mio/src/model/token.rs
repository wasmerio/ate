use std::fmt;
use std::str::FromStr;
use serde::*;
use ate_crypto::ChainKey;
use ate_crypto::SerializationFormat;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkToken {
    pub chain: ChainKey,
    pub access_token: String,
}

impl fmt::Display
for NetworkToken
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes = SerializationFormat::MessagePack.serialize(self).unwrap();
        let bytes = base64::encode(bytes);
        write!(f, "{}", bytes)
    }
}

impl FromStr
for NetworkToken
{
    type Err = ate_crypto::error::SerializationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = base64::decode(s)?;
        Ok(SerializationFormat::MessagePack.deserialize(&bytes[..])?)
    }
}
