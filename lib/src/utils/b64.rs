use serde::{Serialize, Deserialize};
use serde::{Serializer, de::Deserializer};

pub fn vec_as_base64<S>(data: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where S: Serializer
{
    serializer.serialize_str(&base64::encode(&data[..]))
}

pub fn vec_from_base64<'a, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where D: Deserializer<'a>
{
    use serde::de::Error;
    let ret = String::deserialize(deserializer)
            .and_then(|string| base64::decode(&string)
            .map_err(|err| Error::custom(err.to_string()))
    )?;
    Ok(ret)
}