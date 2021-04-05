use std::io::stdout;
use std::io::Write;
use url::Url;
use std::time::Duration;

use ate::prelude::*;

use crate::conf_auth;
use crate::prelude::*;
use crate::commands::*;

#[allow(dead_code)]
pub async fn login_command(username: String, password: String, code: Option<String>, auth: Url) -> Result<AteSession, LoginError>
{
    // Open a command chain
    let chain_url = crate::helper::command_url(auth);
    let registry = ate::mesh::Registry::new(&conf_auth()).await;
    let chain = registry.open(&chain_url).await?;

    // Generate a read-key using the password and some seed data
    // (this read-key will be mixed with entropy on the server side to decrypt the row
    //  which means that neither the client nor the server can get at the data alone)
    let prefix = format!("remote-login:{}:", username);
    let read_key = super::password_to_read_key(&prefix, &password, 10);

    // Create the session
    let mut session = AteSession::new(&conf_auth());
    
    // Create the login command
    let login = LoginRequest {
        email: username.clone(),
        secret: read_key,
        code,
    };

    // Attempt the login request with a 10 second timeout
    let response: LoginResponse = chain.invoke_ext(&session, login, Duration::from_secs(10)).await?;
    match response {
        LoginResponse::AccountLocked => Err(LoginError::AccountLocked),
        LoginResponse::NotFound => Err(LoginError::NotFound(username)),
        LoginResponse::Success {
            mut authority
        } => {
            session.properties.append(&mut authority);
            Ok(session)
        }
    }
}

pub async fn load_credentials(username: String, read_key: EncryptKey, _code: Option<String>, auth: Url) -> Result<AteSession, AteError>
{
    // Prepare for the load operation
    let key = PrimaryKey::from(username.clone());
    let mut session = AteSession::new(&conf_auth());
    session.add_read_key(&read_key);

    // Compute which chain our user exists in
    let chain_url = crate::helper::auth_url(auth, &username);

    // Generate a chain key that matches this username on the authentication server
    let registry = ate::mesh::Registry::new(&conf_auth()).await;
    let chain = registry.open(&chain_url).await?;

    // Load the user
    let mut dio = chain.dio(&session).await;
    let user = dio.load::<User>(&key).await?;

    // Build a new session
    let mut session = AteSession::new(&conf_auth());
    for access in user.access.iter() {
        if let Some(read) = &access.read {
            session.add_read_key(read);
        }
        if let Some(write) = &access.write {
            session.add_write_key(write);
        }
    }
    Ok(session)
}

pub async fn main_login(
    username: Option<String>,
    password: Option<String>,
    code: Option<String>,
    auth: Url
) -> Result<AteSession, LoginError>
{
    let username = match username {
        Some(a) => a,
        None => {
            print!("Username: ");
            stdout().lock().flush()?;
            let mut s = String::new();
            std::io::stdin().read_line(&mut s).expect("Did not enter a valid username");
            s
        }
    };

    let password = match password {
        Some(a) => a,
        None => {
            print!("Password: ");
            stdout().lock().flush()?;
            rpassword::read_password().unwrap()
        }
    };

    // Read the code
    let code = match code {
        Some(a) => a,
        None => {
            print!("Code: ");
            stdout().lock().flush()?;
            let mut s = String::new();
            std::io::stdin().read_line(&mut s).expect("Did not enter a valid username");
            s
        }
    };

    // Login using the authentication server which will give us a session with all the tokens
    Ok(login_command(username, password, Some(code), auth).await?)
}