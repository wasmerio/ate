#[allow(unused_imports)]
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::crypto::*;

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AteSessionProperty {
    None,
    ReadKey(EncryptKey),
    PrivateReadKey(PrivateEncryptKey),
    PublicReadKey(PublicEncryptKey),
    WriteKey(PrivateSignKey),
    Uid(u32),
    Gid(u32),
}

impl Default for AteSessionProperty {
    fn default() -> Self {
        AteSessionProperty::None
    }
}

impl std::fmt::Display for AteSessionProperty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AteSessionProperty::None => write!(f, "none"),
            AteSessionProperty::ReadKey(a) => write!(f, "read-key:{}", a),
            AteSessionProperty::PrivateReadKey(a) => write!(f, "private-read-key:{}", a),
            AteSessionProperty::PublicReadKey(a) => write!(f, "public-read-key:{}", a),
            AteSessionProperty::WriteKey(a) => write!(f, "write-key:{}", a),
            AteSessionProperty::Uid(a) => write!(f, "uid:{}", a),
            AteSessionProperty::Gid(a) => write!(f, "gid:{}", a),
        }
    }
}
