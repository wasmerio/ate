#![cfg_attr(not(debug_assertions), allow(dead_code, unused_imports, unused_variables))]
use url::Url;
use ate::prelude::*;
use std::io::Write;
use std::io::stdout;

mod model;
mod helper;

pub use crate::model::*;
pub use helper::chain_url;
pub use helper::conf_auth;
pub use helper::password_to_read_key;

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
    let read_key = password_to_read_key(&username, &password, 1000);

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
    load_credentials(username, read_key, auth).await
}

pub async fn load_credentials(username: String, read_key: EncryptKey, auth: Url) -> Result<AteSession, AteError>
{
    // Prepare for the load operation
    let key = PrimaryKey::from(username.clone());
    let mut session = AteSession::default();
    session.add_read_key(&read_key);

    // Compute which chain our user exists in
    let chain_url = crate::helper::chain_url(auth, &username);

    // Generate a chain key that matches this username on the authentication server
    let mut conf = ConfAte::default();
    conf.configured_for(ConfiguredFor::BestSecurity);
    let registry = ate::mesh::Registry::new(&conf).await;
    let chain = registry.open(&chain_url).await?;

    // Load the user
    let mut dio = chain.dio(&session).await;
    let user = dio.load::<User>(&key).await?;

    // Build a new session
    let mut session = AteSession::default();
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

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
