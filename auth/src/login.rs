use std::io::stdout;
use std::io::Write;
use url::Url;

use ate::prelude::*;

use crate::conf_auth;
use crate::prelude::*;
use crate::commands::*;

#[allow(dead_code)]
pub async fn login_command(username: String, read_key: EncryptKey, code: String, auth: Url) -> Result<User, AteError>
{
    // Open a command chain
    let chain_url = crate::helper::command_url(auth);
    let registry = ate::mesh::Registry::new(&conf_auth()).await;
    let chain = registry.open(&chain_url).await?;

    // Create the session
    let mut session = AteSession::new(&conf_auth());
    session.add_read_key(&read_key);

    // Create the login command
    let login = CmdLogin {
        email: username,
        secret: read_key,
        code,
    };

    // Send the login command
    let mut dio = chain.dio(&session).await;
    dio.store(Cmd::Login(login))?;
    dio.commit().await?;

    // Fail
    Err(AteError::NotImplemented)
}

pub async fn load_credentials(username: String, read_key: EncryptKey, auth: Url) -> Result<AteSession, AteError>
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
) -> Result<AteSession, AteError>
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

    // Compute the read and write keys
    let read_key = super::password_to_read_key(&username, &password, 1000);

    let _code = match code {
        Some(a) => a,
        None => {
            print!("Code: ");
            stdout().lock().flush()?;
            let mut s = String::new();
            std::io::stdin().read_line(&mut s).expect("Did not enter a valid username");
            s
        }
    };

    // Load the user credentials
    Ok(load_credentials(username, read_key, auth).await?)
}