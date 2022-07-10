#![allow(unused_imports)]
use ate::prelude::*;
use error_chain::bail;
use std::io::stdout;
use std::io::Write;
use std::sync::Arc;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use url::Url;

use crate::error::*;
use crate::helper::*;
use crate::opt::*;
use crate::prelude::*;
use crate::request::*;

pub async fn reset_command(
    registry: &Registry,
    email: String,
    new_password: String,
    recovery_key: EncryptKey,
    sudo_code: String,
    sudo_code_2: String,
    auth: Url,
) -> Result<ResetResponse, ResetError> {
    // Open a command chain
    let chain = registry.open_cmd(&auth).await?;

    // Generate a read-key using the password and some seed data
    // (this read-key will be mixed with entropy on the server side to decrypt the row
    //  which means that neither the client nor the server can get at the data alone)
    let prefix = format!("remote-login:{}:", email);
    let new_secret = password_to_read_key(&prefix, &new_password, 15, KeySize::Bit192);

    // Create the query command
    let auth = match auth.domain() {
        Some(a) => a.to_string(),
        None => "ate".to_string(),
    };
    let reset = ResetRequest {
        email,
        auth,
        new_secret,
        recovery_key,
        sudo_code,
        sudo_code_2,
    };

    let response: Result<ResetResponse, ResetFailed> = chain.invoke(reset).await?;
    let result = response?;
    Ok(result)
}

pub async fn main_reset(
    username: Option<String>,
    recovery_code: Option<String>,
    sudo_code: Option<String>,
    sudo_code_2: Option<String>,
    new_password: Option<String>,
    auth: Url,
) -> Result<ResetResponse, ResetError> {
    if recovery_code.is_none() || sudo_code.is_none() {
        eprintln!(
            r#"# Account Reset Process

You will need *both* of the following to reset your account:
- Your 'recovery code' that you saved during account creation - if not - then
  the recovery code is likely still in your email inbox.
- Two sequential 'authenticator code' response challenges from your mobile app.
"#
        );
    }

    let username = match username {
        Some(a) => a,
        None => {
            print!("Username: ");
            stdout().lock().flush()?;
            let mut s = String::new();
            std::io::stdin()
                .read_line(&mut s)
                .expect("Did not enter a valid username");
            s.trim().to_string()
        }
    };

    let recovery_code = match recovery_code {
        Some(a) => a,
        None => {
            print!("Recovery Code: ");
            stdout().lock().flush()?;
            let mut s = String::new();
            std::io::stdin()
                .read_line(&mut s)
                .expect("Did not enter a valid recovery code");
            s.trim().to_string()
        }
    };
    let recovery_prefix = format!("recover-login:{}:", username);
    let recovery_key = password_to_read_key(&recovery_prefix, &recovery_code, 15, KeySize::Bit192);

    let new_password = match new_password {
        Some(a) => a,
        None => {
            let ret1 = rpassword_wasi::prompt_password("New Password: ").unwrap();
            let ret2 = rpassword_wasi::prompt_password("New Password Again: ").unwrap();
            if ret1 != ret2 {
                bail!(ResetErrorKind::PasswordMismatch);
            }

            ret2
        }
    };

    let sudo_code = match sudo_code {
        Some(a) => a,
        None => {
            print!("Authenticator Code: ");
            stdout().lock().flush()?;
            let mut s = String::new();
            std::io::stdin()
                .read_line(&mut s)
                .expect("Did not enter a valid authenticator code");
            s.trim().to_string()
        }
    };

    let sudo_code_2 = match sudo_code_2 {
        Some(a) => a,
        None => {
            print!("Next Authenticator Code: ");
            stdout().lock().flush()?;
            let mut s = String::new();
            std::io::stdin()
                .read_line(&mut s)
                .expect("Did not enter a valid authenticator code");
            s = s.trim().to_string();

            if sudo_code == s {
                bail!(ResetErrorKind::AuthenticatorCodeEqual);
            }

            s
        }
    };

    let registry = ate::mesh::Registry::new(&conf_cmd()).await.cement();
    let result = match reset_command(
        &registry,
        username,
        new_password,
        recovery_key,
        sudo_code,
        sudo_code_2,
        auth,
    )
    .await
    {
        Ok(a) => a,
        Err(err) => {
            bail!(err);
        }
    };

    if is_tty_stdout() {
        println!("Account reset (id={})", result.key);

        // Display the QR code
        println!("");
        if let Some(message_of_the_day) = &result.message_of_the_day {
            println!("{}", message_of_the_day.as_str());
            println!("");
        }
        println!("Below is your new Google Authenticator QR code - scan it on your phone and");
        println!("save it as this code is the only way you can recover the account another time.");
        println!("");
        println!("{}", result.qr_code);
    }

    Ok(result)
}
