use url::Url;

use ate::prelude::*;
use ate::crypto::EncryptKey;
use ate::error::SerializationError;

use crate::model::*;

pub fn auth_url(auth: Url, email: &String) -> Url
{
    let hash = AteHash::from(email.clone());
    let hex = hash.to_hex_string().to_lowercase();
    let mut ret = auth.clone();
    ret.set_path(format!("{}-{}", ret.path(), &hex[..4]).as_str());
    ret
}

pub fn auth_chain_key(path: String, email: &String) -> ChainKey
{
    let hash = AteHash::from(email.clone());
    let hex = hash.to_hex_string().to_lowercase();
    ChainKey::new(format!("{}-{}", path, &hex[..4]))
}

pub fn command_url(auth: Url) -> Url
{
    let hex = PrimaryKey::generate().as_hex_string().to_lowercase();
    let mut ret = auth.clone();
    ret.set_path(format!("cmd-{}", &hex[..16]).as_str());
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

pub(crate) fn compute_user_auth(user: &User, session: AteSession) -> AteSession
{
    let mut session = session.clone();
    for auth in user.access.iter() {
        if let Some(read) = &auth.read {
            session.add_read_key(read);
        }
        if let Some(write) = &auth.write {
            session.add_write_key(write);
        }
    }

    session
}

pub(crate) fn compute_sudo_auth(sudo: &Sudo, session: AteSession) -> AteSession
{
    let mut session = session.clone();
    for auth in sudo.access.iter() {
        if let Some(read) = &auth.read {
            session.add_read_key(read);
        }
        if let Some(write) = &auth.write {
            session.add_write_key(write);
        }
    }

    session
}

pub fn session_to_b64(session: AteSession) -> Result<String, SerializationError>
{
    let format = SerializationFormat::MessagePack;
    let bytes = format.serialize(&session)?;
    Ok(base64::encode(bytes))
}

pub fn b64_to_session(val: String) -> AteSession
{
    let format = SerializationFormat::MessagePack;
    let bytes = base64::decode(val).unwrap();
    format.deserialize( &bytes).unwrap()
}