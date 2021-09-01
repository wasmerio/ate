#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};

use ::ate::prelude::*;
use ::ate::crypto::EncryptKey;

pub fn password_to_read_key(seed: &String, password: &String, repeat: i32, key_size: KeySize) -> EncryptKey
{
    let mut bytes = Vec::from(seed.as_bytes());
    bytes.extend(Vec::from(password.as_bytes()).iter());
    while bytes.len() < 1000 {
        bytes.push(0);
    }
    let hash = AteHash::from_bytes_sha3(password.as_bytes(), repeat);
    EncryptKey::from_seed_bytes(hash.to_bytes(), key_size)
}

pub fn estimate_user_name_as_uid(email: String) -> u32
{
    let min = ((u32::MAX as u64) * 2) / 4;
    let max = ((u32::MAX as u64) * 3) / 4;
    PrimaryKey::from_ext(AteHash::from(email), min as u64, max as u64).as_u64() as u32
}

pub fn estimate_group_name_as_gid(group: String) -> u32
{
    let min = ((u32::MAX as u64) * 3) / 4;
    let max = ((u32::MAX as u64) * 4) / 4;
    PrimaryKey::from_ext(AteHash::from(group), min as u64, max as u64).as_u64() as u32
}

pub fn session_to_b64(session: AteSessionType) -> Result<String, SerializationError>
{
    let format = SerializationFormat::MessagePack;
    let bytes = format.serialize(&session)?;
    Ok(base64::encode(bytes))
}

pub fn b64_to_session(val: String) -> AteSessionType
{
    let val = val.trim().to_string();
    let format = SerializationFormat::MessagePack;
    let bytes = base64::decode(val).unwrap();
    format.deserialize( &bytes).unwrap()
}

#[allow(dead_code)]
pub fn is_public_domain(domain: &str) -> bool {
    match domain {
        "gmail.com" => true,
        "zoho.com" => true,
        "outlook.com" => true,
        "hotmail.com" => true,
        "mail.com" => true,
        "yahoo.com" => true,
        "gmx.com" => true,
        "hushmail.com" => true,
        "hush.com" => true,
        "inbox.com" => true,
        "aol.com" => true,
        "yandex.com" => true,
        _ => false
    }
}