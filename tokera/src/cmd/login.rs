use ate::error::AteError;
use ate::prelude::*;
use std::fs::File;
use std::io::Write;

#[cfg(unix)]
use std::env::temp_dir;
#[cfg(unix)]
use std::os::unix::fs::symlink;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use ate_auth::cmd::*;
use ate_auth::helper::*;

use crate::opt::*;

pub async fn main_opts_login(
    action: OptsLogin,
    token_path: String,
    auth: url::Url,
) -> Result<(), AteError> {
    // Convert the token path to a real path
    let token_path = shellexpand::tilde(&token_path).to_string();

    // If a token was supplied then just use it, otherwise we need to get one
    let token = if let Some(token) = action.token {
        token
    } else {
        // Get the token session
        let session = main_login(action.email, action.password, auth.clone()).await?;
        let session: AteSessionType = if action.sudo {
            main_sudo(session, None, auth).await?.into()
        } else {
            session.into()
        };
        session_to_b64(session).unwrap()
    };    

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
        let mut file = File::create(save_path.clone())?;

        // Set the permissions so no one else can read it but the current user
        #[cfg(unix)]
        {
            let mut perms = std::fs::metadata(save_path.clone())?.permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(save_path.clone(), perms)?;
        }

        // Write the token to it
        file.write_all(token.as_bytes())?;
    }

    // Update the token path so that it points to this temporary token
    #[cfg(unix)]
    symlink(save_path, token_path)?;

    Ok(())
}

#[cfg(unix)]
fn random_file() -> String {
    let mut tmp = temp_dir();

    let rnd = ate::prelude::PrimaryKey::default().as_hex_string();

    let file_name = format!("{}", rnd);
    tmp.push(file_name);

    let tmp_str = tmp.into_os_string().into_string().unwrap();
    let tmp_str = shellexpand::tilde(&tmp_str).to_string();

    tmp_str
}
