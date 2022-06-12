use std::{fmt::Display, str::FromStr};

use crate::CallError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SerializationFormat {
    Raw,
    #[cfg(feature = "enable_bincode")]
    Bincode,
    #[cfg(feature = "enable_mpack")]
    MessagePack,
    #[cfg(feature = "enable_json")]
    Json,
    #[cfg(feature = "enable_yaml")]
    Yaml,
    #[cfg(feature = "enable_xml")]
    Xml,
    #[cfg(feature = "enable_rkyv")]
    Rkyv
}

impl FromStr for SerializationFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "raw" => Ok(SerializationFormat::Raw),
            #[cfg(feature = "enable_bincode")]
            "bincode" => Ok(SerializationFormat::Bincode),
            #[cfg(feature = "enable_mpack")]
            "mpack" | "messagepack" => Ok(SerializationFormat::MessagePack),
            #[cfg(feature = "enable_json")]
            "json" => Ok(SerializationFormat::Json),
            #[cfg(feature = "enable_json")]
            "yaml" => Ok(SerializationFormat::Yaml),
            #[cfg(feature = "enable_yaml")]
            "xml" => Ok(SerializationFormat::Xml),
            #[cfg(feature = "enable_rkyv")]
            "rkyv" => Ok(SerializationFormat::Rkyv),
            _ => {
                let mut msg = "valid serialization formats are".to_string();
                msg.push_str(" 'raw'");
                #[cfg(feature = "enable_bincode")]
                msg.push_str(", 'bincode'");
                #[cfg(feature = "enable_mpack")]
                msg.push_str(", 'mpack'");
                #[cfg(feature = "enable_json")]
                msg.push_str(", 'json'");
                #[cfg(feature = "enable_yaml")]
                msg.push_str(", 'yaml'");
                #[cfg(feature = "enable_xml")]
                msg.push_str(", 'xml'");
                #[cfg(feature = "enable_rkyv")]
                msg.push_str(", 'rkyv'");
                msg.push_str(".");
                return Err(msg);
            }
        }
    }
}

impl Display for SerializationFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SerializationFormat::Raw => write!(f, "raw"),
            #[cfg(feature = "enable_bincode")]
            SerializationFormat::Bincode => write!(f, "bincode"),
            #[cfg(feature = "enable_mpack")]
            SerializationFormat::MessagePack => write!(f, "mpack"),
            #[cfg(feature = "enable_json")]
            SerializationFormat::Json => write!(f, "json"),
            #[cfg(feature = "enable_yaml")]
            SerializationFormat::Yaml => write!(f, "yaml"),
            #[cfg(feature = "enable_xml")]
            SerializationFormat::Xml => write!(f, "xml"),
            #[cfg(feature = "enable_rkyv")]
            SerializationFormat::Rkyv => write!(f, "rkyv"),
        }
    }
}

impl SerializationFormat
{
    #[cfg(feature = "enable_rkyv")]
    pub fn deserialize<T>(&self, mut data: Vec<u8>) -> Result<T, CallError>
    where T: serde::de::DeserializeOwned,
          T: rkyv::Archive,
          T::Archived: rkyv::Deserialize<T, rkyv::de::deserializers::SharedDeserializeMap>
    {
        use SerializationFormat::*;
        Ok(
            match self {
                Raw => {
                    if std::any::type_name::<Vec<u8>>() == std::any::type_name::<T>() {
                        unsafe {
                            let r = std::mem::transmute_copy(&data);
                            std::mem::forget(
                                std::mem::replace(&mut data, Vec::new())
                            );
                            r
                        }
                    } else {
                        return Err(CallError::DeserializationFailed)
                    }
                }
                #[cfg(feature = "enable_bincode")]
                Bincode => bincode::deserialize::<T>(data.as_ref())
                    .map_err(|_err| CallError::DeserializationFailed)?,
                #[cfg(feature = "enable_mpack")]
                MessagePack => rmp_serde::from_read_ref(&data[..])
                    .map_err(|_err| CallError::DeserializationFailed)?,
                #[cfg(feature = "enable_json")]
                Json => serde_json::from_slice::<T>(data.as_ref())
                    .map_err(|_err| CallError::DeserializationFailed)?,
                #[cfg(feature = "enable_yaml")]
                Yaml => serde_yaml::from_slice(data.as_ref())
                    .map_err(|_err| CallError::DeserializationFailed)?,
                #[cfg(feature = "enable_xml")]
                Xml => serde_xml_rs::from_reader(&data[..])
                    .map_err(|_err| CallError::DeserializationFailed)?,
                #[cfg(feature = "enable_xml")]
                Rkyv => unsafe {
                    rkyv::from_bytes_unchecked(&data[..])
                        .map_err(|_err| CallError::DeserializationFailed)?
                },
            }
        )
    }

    #[cfg(feature = "enable_rkyv")]
    pub fn serialize<T, const N: usize>(&self, mut data: T) -> Result<Vec<u8>, CallError>
    where T: serde::Serialize,
          T: rkyv::Serialize<rkyv::ser::serializers::AllocSerializer<N>>
    {
        use SerializationFormat::*;
        Ok(
            match self {
                Raw => {
                    if std::any::type_name::<Vec<u8>>() == std::any::type_name::<T>() {
                        unsafe {
                            let r = std::mem::transmute_copy(&data);
                            let ptr = &mut data as *mut T;
                            let ptr = ptr as *mut ();
                            let ptr = ptr as *mut Vec<u8>;
                            let ptr = &mut *ptr;
                            std::mem::forget(
                                std::mem::replace(ptr, Vec::new())
                            );
                            r
                        }
                    } else {
                        return Err(CallError::SerializationFailed)
                    }
                }
                #[cfg(feature = "enable_bincode")]
                Bincode => bincode::serialize::<T>(&data)
                    .map_err(|_err| CallError::SerializationFailed)?,
                #[cfg(feature = "enable_mpack")]
                MessagePack => rmp_serde::to_vec(&data)
                    .map_err(|_err| CallError::SerializationFailed)?,
                #[cfg(feature = "enable_json")]
                Json => serde_json::to_vec::<T>(&data)
                    .map_err(|_err| CallError::SerializationFailed)?,
                #[cfg(feature = "enable_yaml")]
                Yaml => serde_yaml::to_vec(&data)
                    .map_err(|_err| CallError::SerializationFailed)?,
                #[cfg(feature = "enable_xml")]
                Xml => {
                    let mut ret = Vec::new();
                    serde_xml_rs::to_writer(&mut ret, &data)
                        .map_err(|_err| CallError::SerializationFailed)?;
                    ret
                }
                Rkyv => rkyv::to_bytes(&data)
                    .map(|ret| ret.into_vec())
                    .map_err(|_err| CallError::SerializationFailed)?,
            }
        )
    }

    #[cfg(not(feature = "enable_rkyv"))]
    pub fn deserialize<T>(&self, mut data: Vec<u8>) -> Result<T, CallError>
    where T: serde::de::DeserializeOwned
    {
        use SerializationFormat::*;
        Ok(
            match self {
                Raw => {
                    if std::any::type_name::<Vec<u8>>() == std::any::type_name::<T>() {
                        unsafe {
                            let r = std::mem::transmute_copy(&data);
                            std::mem::forget(
                                std::mem::replace(&mut data, Vec::new())
                            );
                            r
                        }
                    } else {
                        return Err(CallError::DeserializationFailed)
                    }
                }
                #[cfg(feature = "enable_bincode")]
                Bincode => bincode::deserialize::<T>(data.as_ref())
                    .map_err(|_err| CallError::DeserializationFailed)?,
                #[cfg(feature = "enable_mpack")]
                MessagePack => rmp_serde::from_read_ref(&data[..])
                    .map_err(|_err| CallError::DeserializationFailed)?,
                #[cfg(feature = "enable_json")]
                Json => serde_json::from_slice::<T>(data.as_ref())
                    .map_err(|_err| CallError::DeserializationFailed)?,
                #[cfg(feature = "enable_yaml")]
                Yaml => serde_yaml::from_slice(data.as_ref())
                    .map_err(|_err| CallError::DeserializationFailed)?,
                #[cfg(feature = "enable_xml")]
                Xml => serde_xml_rs::from_reader(&data[..])
                    .map_err(|_err| CallError::DeserializationFailed)?,
            }
        )
    }    

    #[cfg(not(feature = "enable_rkyv"))]
    pub fn serialize<T>(&self, mut data: T) -> Result<Vec<u8>, CallError>
    where T: serde::Serialize
    {
        use SerializationFormat::*;
        Ok(
            match self {
                Raw => {
                    if std::any::type_name::<Vec<u8>>() == std::any::type_name::<T>() {
                        unsafe {
                            let r = std::mem::transmute_copy(&data);
                            let ptr = &mut data as *mut T;
                            let ptr = ptr as *mut ();
                            let ptr = ptr as *mut Vec<u8>;
                            let ptr = &mut *ptr;
                            std::mem::forget(
                                std::mem::replace(ptr, Vec::new())
                            );
                            r
                        }
                    } else {
                        return Err(CallError::SerializationFailed)
                    }
                }
                #[cfg(feature = "enable_bincode")]
                Bincode => bincode::serialize::<T>(&data)
                    .map_err(|_err| CallError::SerializationFailed)?,
                #[cfg(feature = "enable_mpack")]
                MessagePack => rmp_serde::to_vec(&data)
                    .map_err(|_err| CallError::SerializationFailed)?,
                #[cfg(feature = "enable_json")]
                Json => serde_json::to_vec::<T>(&data)
                    .map_err(|_err| CallError::SerializationFailed)?,
                #[cfg(feature = "enable_yaml")]
                Yaml => serde_yaml::to_vec(&data)
                    .map_err(|_err| CallError::SerializationFailed)?,
                #[cfg(feature = "enable_xml")]
                Xml => {
                    let mut ret = Vec::new();
                    serde_xml_rs::to_writer(&mut ret, &data)
                        .map_err(|_err| CallError::SerializationFailed)?;
                    ret
                }
            }
        )
    }
}
