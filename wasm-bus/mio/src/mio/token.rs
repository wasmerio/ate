use std::io::Write;
use ate_crypto::SerializationFormat;
use ate_crypto::error::SerializationError;
#[cfg(unix)]
use std::{
    env::temp_dir,
    os::unix::fs::{
        PermissionsExt,
        symlink
    }
};

use crate::model::NetworkToken;

pub fn decode_access_token(token: String) -> Result<NetworkToken, SerializationError> {
    let val = token.trim().to_string();
    let bytes = base64::decode(val).unwrap();
    Ok(SerializationFormat::MessagePack.deserialize(&bytes)?)
}

pub fn load_access_token(token_path: String) -> Result<Option<NetworkToken>, SerializationError> {
    let token_path = format!("{}.network", token_path);
    let token_path = shellexpand::tilde(token_path.as_str()).to_string();
    if let Ok(token) = std::fs::read_to_string(token_path) {
        Ok(Some(decode_access_token(token)?))
    } else {
        Ok(None)
    }
}

pub fn encode_access_token(token: &NetworkToken) -> Result<String, SerializationError> {
    let bytes = SerializationFormat::MessagePack.serialize(&token)?;
    let bytes = base64::encode(bytes);
    Ok(bytes)
}

pub async fn save_access_token(token_path: String, token: &NetworkToken) -> Result<(), SerializationError> {
    let bytes = encode_access_token(token)?;
    
    let token_path = format!("{}.network", token_path);
    let token_path = shellexpand::tilde(token_path.as_str()).to_string();

    // Remove any old paths
    if let Ok(old) = std::fs::canonicalize(token_path.clone()) {
        let _ = std::fs::remove_file(old);
    }
    let _ = std::fs::remove_file(token_path.clone());

    // Create the folder structure
    let path = std::path::Path::new(&token_path);
    let _ = std::fs::create_dir_all(path.parent().unwrap().clone());

    // Create a random file that will hold the token
    #[cfg(unix)]
    let save_path = random_file();
    #[cfg(not(unix))]
    let save_path = token_path;

    {
        // Create the folder structure
        let path = std::path::Path::new(&save_path);
        let _ = std::fs::create_dir_all(path.parent().unwrap().clone());

        // Create the file
        let mut file = std::fs::File::create(save_path.clone())?;

        // Set the permissions so no one else can read it but the current user
        #[cfg(unix)]
        {
            let mut perms = std::fs::metadata(save_path.clone())?.permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(save_path.clone(), perms)?;
        }

        // Write the token to it
        file.write_all(bytes.as_bytes())?;
    }

    // Update the token path so that it points to this temporary token
    #[cfg(unix)]
    symlink(save_path, token_path)?;
    Ok(())
}

pub async fn clear_access_token(token_path: String) {
    let token_path = format!("{}.network", token_path);
    let token_path = shellexpand::tilde(token_path.as_str()).to_string();

    // Remove any old paths
    if let Ok(old) = std::fs::canonicalize(token_path.clone()) {
        let _ = std::fs::remove_file(old);
    }
    let _ = std::fs::remove_file(token_path.clone());
}

#[cfg(unix)]
pub fn random_file() -> String {
    let mut tmp = temp_dir();

    let rnd = ate_crypto::PrimaryKey::default().as_hex_string();

    let file_name = format!("{}", rnd);
    tmp.push(file_name);

    let tmp_str = tmp.into_os_string().into_string().unwrap();
    let tmp_str = shellexpand::tilde(&tmp_str).to_string();

    tmp_str
}
