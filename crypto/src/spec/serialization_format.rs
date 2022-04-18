use serde::*;
use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;

use crate::error::SerializationError;

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Copy,
    Eq,
    PartialEq,
    PartialOrd,
    Ord,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum SerializationFormat {
    Json = 1,
    MessagePack = 2,
    Bincode = 3,
}

impl std::str::FromStr for SerializationFormat {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "json" => Ok(SerializationFormat::Json),
            "messagepack" => Ok(SerializationFormat::MessagePack),
            "mpack" => Ok(SerializationFormat::MessagePack),
            "bincode" => Ok(SerializationFormat::Bincode),
            "bc" => Ok(SerializationFormat::Bincode),
            _ => Err("valid values are 'json', 'messagepack'/'mpack', 'bincode'/'bc'"),
        }
    }
}

impl SerializationFormat {
    pub fn iter() -> std::vec::IntoIter<SerializationFormat> {
        vec![
            SerializationFormat::Json,
            SerializationFormat::MessagePack,
            SerializationFormat::Bincode,
        ]
        .into_iter()
    }

    pub fn serialize<T>(&self, val: &T) -> Result<Vec<u8>, SerializationError>
    where
        T: Serialize + ?Sized,
    {
        match self {
            SerializationFormat::Json => Ok(serde_json::to_vec(val)?),
            SerializationFormat::MessagePack => Ok(rmp_serde::to_vec(val)?),
            SerializationFormat::Bincode => Ok(bincode::serialize(val)?),
        }
    }

    pub fn deserialize<'a, T>(&self, val: &'a [u8]) -> Result<T, SerializationError>
    where
        T: serde::de::Deserialize<'a>,
    {
        match self {
            SerializationFormat::Json => Ok(serde_json::from_slice(val)?),
            SerializationFormat::MessagePack => Ok(rmp_serde::from_read_ref(val)?),
            SerializationFormat::Bincode => Ok(bincode::deserialize(val)?),
        }
    }
}

impl std::fmt::Display for SerializationFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SerializationFormat::Bincode => write!(f, "bincode"),
            SerializationFormat::Json => write!(f, "json"),
            SerializationFormat::MessagePack => write!(f, "mpack"),
        }
    }
}