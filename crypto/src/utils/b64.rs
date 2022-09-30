#![allow(unused_imports)]
use std::convert::TryInto;

use serde::{de::Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

pub fn vec_serialize<S>(data: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if std::any::type_name::<S>().contains("serde_json") {
        serializer.serialize_str(&base64::encode(&data[..]))
    } else {
        <Vec<u8>>::serialize(data, serializer)
    }
}

pub fn vec_deserialize<'a, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'a>,
{
    if std::any::type_name::<D>().contains("serde_json") {
        use serde::de::Error;
        let ret = String::deserialize(deserializer).and_then(|string| {
            base64::decode(&string).map_err(|err| Error::custom(err.to_string()))
        })?;
        Ok(ret)
    } else {
        <Vec<u8>>::deserialize(deserializer)
    }
}

pub fn b16_serialize<S>(data: &[u8; 16], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if std::any::type_name::<S>().contains("serde_json") {
        serializer.serialize_str(&base64::encode(&data[..]))
    } else {
        <[u8; 16]>::serialize(data, serializer)
    }
}

pub fn b16_deserialize<'a, D>(deserializer: D) -> Result<[u8; 16], D::Error>
where
    D: Deserializer<'a>,
{
    if std::any::type_name::<D>().contains("serde_json") {
        use serde::de::Error;
        let ret = String::deserialize(deserializer).and_then(|string| {
            base64::decode(&string).map_err(|err| Error::custom(err.to_string()))
        })?;
        ret.try_into().map_err(|e: Vec<u8>| {
            Error::custom(format!("expected 16 bytes but found {}", e.len()).as_str())
        })
    } else {
        <[u8; 16]>::deserialize(deserializer)
    }
}

pub fn b24_serialize<S>(data: &[u8; 24], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if std::any::type_name::<S>().contains("serde_json") {
        serializer.serialize_str(&base64::encode(&data[..]))
    } else {
        <[u8; 24]>::serialize(data, serializer)
    }
}

pub fn b24_deserialize<'a, D>(deserializer: D) -> Result<[u8; 24], D::Error>
where
    D: Deserializer<'a>,
{
    if std::any::type_name::<D>().contains("serde_json") {
        use serde::de::Error;
        let ret = String::deserialize(deserializer).and_then(|string| {
            base64::decode(&string).map_err(|err| Error::custom(err.to_string()))
        })?;
        ret.try_into().map_err(|e: Vec<u8>| {
            Error::custom(format!("expected 24 bytes but found {}", e.len()).as_str())
        })
    } else {
        <[u8; 24]>::deserialize(deserializer)
    }
}

pub fn b32_serialize<S>(data: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if std::any::type_name::<S>().contains("serde_json") {
        serializer.serialize_str(&base64::encode(&data[..]))
    } else {
        <[u8; 32]>::serialize(data, serializer)
    }
}

pub fn b32_deserialize<'a, D>(deserializer: D) -> Result<[u8; 32], D::Error>
where
    D: Deserializer<'a>,
{
    if std::any::type_name::<D>().contains("serde_json") {
        use serde::de::Error;
        let ret = String::deserialize(deserializer).and_then(|string| {
            base64::decode(&string).map_err(|err| Error::custom(err.to_string()))
        })?;
        ret.try_into().map_err(|e: Vec<u8>| {
            Error::custom(format!("expected 32 bytes but found {}", e.len()).as_str())
        })
    } else {
        <[u8; 32]>::deserialize(deserializer)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct TestClass {
    pub header: String,
    #[serde(serialize_with = "vec_serialize", deserialize_with = "vec_deserialize")]
    pub my_bytes: Vec<u8>,
    #[serde(serialize_with = "b16_serialize", deserialize_with = "b16_deserialize")]
    pub my_b1: [u8; 16],
    #[serde(serialize_with = "b24_serialize", deserialize_with = "b24_deserialize")]
    pub my_b2: [u8; 24],
    #[serde(serialize_with = "b32_serialize", deserialize_with = "b32_deserialize")]
    pub my_b3: [u8; 32],
}

#[test]
fn test_b64() {
    crate::utils::bootstrap_test_env();

    let plain = TestClass {
        header: "ate".to_string(),
        my_bytes: vec![
            112u8, 84u8, 99u8, 210u8, 55u8, 201u8, 202u8, 203u8, 204u8, 205u8, 206u8, 207u8,
        ],
        my_b1: [
            1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8, 9u8, 10u8, 11u8, 12u8, 13u8, 14u8, 15u8, 16u8,
        ],
        my_b2: [
            1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8, 9u8, 10u8, 11u8, 12u8, 13u8, 14u8, 15u8, 16u8,
            1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8,
        ],
        my_b3: [
            1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8, 9u8, 10u8, 11u8, 12u8, 13u8, 14u8, 15u8, 16u8,
            1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8, 9u8, 10u8, 11u8, 12u8, 13u8, 14u8, 15u8, 16u8,
        ],
    };

    let cipher = bincode::serialize(&plain).unwrap();
    trace!("{:?}", cipher);
    let test: TestClass = bincode::deserialize(&cipher[..]).unwrap();
    assert_eq!(test, plain);

    let cipher = rmp_serde::to_vec(&plain).unwrap();
    trace!("{:?}", cipher);
    let test: TestClass = rmp_serde::from_read_ref(&cipher[..]).unwrap();
    assert_eq!(test, plain);

    let cipher = serde_json::to_string_pretty(&plain).unwrap();
    trace!("{}", cipher);
    let test: TestClass = serde_json::from_str(&cipher).unwrap();
    assert_eq!(test, plain);
}
