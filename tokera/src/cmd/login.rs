use ate::error::AteError;
use ate::prelude::*;

use ate_auth::cmd::*;
use ate_auth::helper::*;

#[cfg(target_os = "wasi")]
use wasm_bus_process::prelude::*;

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

    // Read the session
    let session = b64_to_session(token.clone());
    #[allow(unused)]
    let identity = session.identity();

    // Save the token
    save_token(token, token_path)?;

    // If we are in WASM mode and there is a login script then run it
    #[cfg(target_os = "wasi")]
    if std::path::Path::new("/etc/login.sh").exists() == true {
        Command::new("export")
            .args(&[format!("USER={}", identity).as_str()])
            .execute()
            .await?;

        Command::new("source")
            .args(&["/etc/login.sh"])
            .execute()
            .await?;
    }
    #[cfg(target_os = "wasi")]
    if std::path::Path::new("/usr/etc/login.sh").exists() == true {
        Command::new("export")
            .args(&[format!("USER={}", identity).as_str()])
            .execute()
            .await?;
            
        Command::new("source")
            .args(&["/usr/etc/login.sh"])
            .execute()
            .await?;
    }

    Ok(())
}
