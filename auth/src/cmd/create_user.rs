#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use error_chain::bail;
use ate::prelude::*;
use std::sync::Arc;
use url::Url;
use std::io::stdout;
use std::io::Write;

use crate::prelude::*;
use crate::helper::*;
use crate::error::*;
use crate::request::*;
use crate::opt::*;

#[allow(dead_code)]
pub async fn create_user_command(registry: &Registry, username: String, password: String, auth: Url, accepted_terms: Option<String>) -> Result<CreateUserResponse, CreateError>
{
    // Open a command chain
    let chain = registry.open_cmd(&auth).await?;

    // Generate a read-key using the password and some seed data
    // (this read-key will be mixed with entropy on the server side to decrypt the row
    //  which means that neither the client nor the server can get at the data alone)
    let prefix = format!("remote-login:{}:", username);
    let read_key = password_to_read_key(&prefix, &password, 15, KeySize::Bit192);
    
    // Create the login command
    let auth = match auth.domain() {
        Some(a) => a.to_string(),
        None => "ate".to_string(),
    };
    let request = CreateUserRequest {
        auth,
        email: username.clone(),
        secret: read_key,
        accepted_terms,
    };

    // Attempt the login request with a 10 second timeout
    let response: Result<CreateUserResponse, CreateUserFailed> = chain.invoke(request).await?;
    let result = response?;
    debug!("key: {}", result.key);
    Ok(result)
}

pub async fn main_create_user(
    username: Option<String>,
    password: Option<String>,
    auth: Url
) -> Result<CreateUserResponse, CreateError>
{
    let username = match username {
        Some(a) => a,
        None => {
            #[cfg(not(feature = "force_tty"))]
            if !is_tty_stdin() {
                bail!(CreateErrorKind::InvalidArguments);
            }

            print!("Username: ");
            stdout().lock().flush()?;
            let mut s = String::new();
            std::io::stdin().read_line(&mut s).expect("Did not enter a valid username");
            s.trim().to_string()
        }
    };

    let password = match password {
        Some(a) => a,
        None => {
            #[cfg(not(feature = "force_tty"))]
            if !is_tty_stdin() {
                bail!(CreateErrorKind::InvalidArguments);
            }

            let ret1 = rpassword_wasi::prompt_password("Password: ").unwrap();

            stdout().lock().flush()?;
            let ret2 = rpassword_wasi::prompt_password("Password Again: ").unwrap();

            if ret1 != ret2 {
                bail!(CreateErrorKind::PasswordMismatch);
            }

            ret2
        }
    };

    // Create a user using the authentication server which will give us a session with all the tokens
    let registry = ate::mesh::Registry::new( &conf_cmd()).await.cement();
    let result = match create_user_command(
        &registry,
        username.clone(),
        password.clone(),
        auth.clone(),
        None
    ).await {
        Ok(a) => a,
        Err(CreateError(CreateErrorKind::AlreadyExists(msg), _)) =>
        {
            eprintln!("{}", msg);
            std::process::exit(1);
        }
        Err(CreateError(CreateErrorKind::TermsAndConditions(terms), _)) =>
        {
            if !is_tty_stdin() {
                bail!(CreateErrorKind::InvalidArguments);
            }

            // We need an agreement to the terms and conditions from the caller
            println!("");
            println!("{}", terms);
            println!("");
            println!("If you agree to the above terms and conditions then type the word 'agree' below");
            
            let mut s = String::new();
            std::io::stdin().read_line(&mut s).expect("Did not enter a valid response");
            let agreement = s.trim().to_string().to_lowercase();
            if agreement != "agree" {
                eprintln!("You may only create an account by specifically agreeing to the terms");
                eprintln!("and conditions laid out above - this can only be confirmed if you");
                eprintln!("specifically type the word 'agree' which you did not enter hence");
                eprintln!("an account can not be created. If this is a mistake then please");
                eprintln!("try again.");
                std::process::exit(1);
            }

            // Try again but this time with an aggrement to the terms and conditions
            create_user_command(&registry, username, password, auth, Some(terms)).await?
        },
        Err(err) => {
            bail!(err);
        }
    };

    if is_tty_stdout() {
        println!("User created (id={})", result.key);

        // Display the QR code
        println!("");
        if let Some(message_of_the_day) = &result.message_of_the_day {
            println!("{}", message_of_the_day.as_str());
            println!("");
        }
        println!("Below is your Google Authenticator QR code - scan it on your phone and");
        println!("save it as this code is the only way you can recover the account.");
        println!("");
        println!("{}", result.qr_code);
    }

    Ok(result)
}