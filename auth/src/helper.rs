use url::Url;

use ate::prelude::*;
use ate::crypto::EncryptKey;

pub fn auth_url(auth: Url, email: &String) -> Url
{
    let hash = AteHash::from(email.clone());
    let hex = hash.to_hex_string().to_lowercase();
    let mut ret = auth.clone();
    ret.set_path(format!("{}-{}", ret.path(), &hex[..4]).as_str());
    ret
}

pub fn command_url(auth: Url) -> Url
{
    let hex = PrimaryKey::generate().as_hex_string().to_lowercase();
    let mut ret = auth.clone();
    ret.set_path(format!("cmd-{}", &hex[..4]).as_str());
    ret
}

pub fn password_to_read_key(seed: &String, password: &String, repeat: i32) -> EncryptKey
{
    let mut bytes = Vec::from(seed.as_bytes());
    bytes.extend(Vec::from(password.as_bytes()).iter());
    while bytes.len() < 1000 {
        bytes.push(0);
    }
    let hash = AteHash::from_bytes_sha3(password.as_bytes(), repeat);
    EncryptKey::from_seed_bytes(hash.to_bytes(), KeySize::Bit256)
}

pub fn conf_auth() -> ConfAte
{
    let mut cfg_ate = ConfAte::default();
    cfg_ate.configured_for(ConfiguredFor::BestSecurity);
    cfg_ate.log_format.meta = SerializationFormat::Json;
    cfg_ate.log_format.data = SerializationFormat::Json;
    cfg_ate.wire_format = SerializationFormat::Json;
    cfg_ate
}