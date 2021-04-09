#![allow(unused_imports)]
use log::{info, error, debug};
use std::io::stdout;
use std::io::Write;
use url::Url;
use std::ops::Deref;
use qrcode::QrCode;
use qrcode::render::unicode;

use ate::prelude::*;
use ate::error::LoadError;

use crate::conf_auth;
use crate::prelude::*;
use crate::commands::*;
use crate::service::AuthService;
use crate::helper::*;
use crate::error::*;
use crate::helper::*;

impl AuthService
{
    pub async fn process_create<'a>(&self, request: CreateRequest, context: InvocationContext<'a>) -> Result<CreateResponse, ServiceError<CreateFailed>>
    {
        info!("create user: {}", request.email);

        // Compute the super_key
        let super_key = match self.compute_super_key(request.secret) {
            Some(a) => a,
            None => { return Err(ServiceError::Reply(CreateFailed::NoMasterKey)); }
        };

        // Compute which chain the user should exist within
        let user_chain_key = auth_chain_key("auth".to_string(), &request.email);
        let chain = context.repository.open_by_key(&user_chain_key).await?;
        let mut dio = chain.dio(&self.master_session).await;

        // If it already exists then fail
        let user_key = PrimaryKey::from(request.email.clone());
        if dio.exists(&user_key).await {
            return Err(ServiceError::Reply(CreateFailed::AlreadyExists));
        }

        // Create the access object
        let read_key = super_key;
        let write_key = PrivateSignKey::generate(KeySize::Bit256);
        let mut access = Vec::new();
        access.push(Authorization {
            name: "loopback".to_string(),
            read: Some(read_key.clone()),
            write: Some(write_key.clone())
        });

        // Create the user and save it
        let user = User {
            person: DaoRef::default(),
            account: DaoRef::default(),
            role: UserRole::Human,
            status: UserStatus::Unverified,
            last_login: None,
            access: access,
            foreign: DaoForeign::default(),
            sudo: DaoRef::default()
        };
        let mut user = dio.prep(user)?;
        
        // Set the authorizations amd commit the user to the tree
        user.auth_mut().read = ReadOption::Specific(read_key.hash());
        user.auth_mut().write = WriteOption::Specific(write_key.hash());
        user.commit(&mut dio)?;

        // Generate a QR code and other sudo keys
        let google_auth = google_authenticator::GoogleAuthenticator::new();
        let google_auth_secret = google_auth.create_secret(32);
        let sudo_read_key = EncryptKey::generate(KeySize::Bit256);
        let sudo_write_key = PrivateSignKey::generate(KeySize::Bit256);
        let mut sudo_access = Vec::new();
        sudo_access.push(Authorization {
            name: "sudo".to_string(),
            read: Some(sudo_read_key.clone()),
            write: Some(sudo_write_key.clone())
        });

        // Compute the super super key thats used to access the elevated rights object
        let super_super_key = match self.compute_super_key(super_key.clone()) {
            Some(a) => a,
            None => { return Err(ServiceError::Reply(CreateFailed::NoMasterKey)); }
        };

        // Build the QR image
        let qr_code = QrCode::new(google_auth_secret.as_bytes()).unwrap()
            .render::<unicode::Dense1x2>()
            .dark_color(unicode::Dense1x2::Light)
            .light_color(unicode::Dense1x2::Dark)
            .build();

        // Create the sudo object and save it using another elevation of the key
        let sudo = Sudo {
            google_auth: google_auth_secret,
            access: sudo_access,
            qr_code
        };
        let mut sudo = dio.prep(sudo)?;
        sudo.auth_mut().read = ReadOption::Specific(super_super_key.hash());
        sudo.auth_mut().write = WriteOption::Specific(sudo_write_key.hash());
        user.sudo.set_id(sudo.key().clone());
        
        // Save the data
        user.commit(&mut dio)?;
        sudo.commit(&mut dio)?;
        dio.commit().await?;

        // Create the authorizations and return them
        let mut session = AteSession::default();
        session.add_read_key(&read_key);
        let session = compute_user_auth(user.deref(), session);

        // Return success to the caller
        Ok(CreateResponse {
            key: user.key().clone(),
            authority: session.properties,
        })
    }
}

#[allow(dead_code)]
pub async fn create_command(username: String, password: String, auth: Url) -> Result<AteSession, CreateError>
{
    // Open a command chain
    let chain_url = crate::helper::command_url(auth);
    let registry = ate::mesh::Registry::new(&conf_auth()).await;
    let chain = registry.open_by_url(&chain_url).await?;

    // Generate a read-key using the password and some seed data
    // (this read-key will be mixed with entropy on the server side to decrypt the row
    //  which means that neither the client nor the server can get at the data alone)
    let prefix = format!("remote-login:{}:", username);
    let read_key = super::password_to_read_key(&prefix, &password, 10);
    
    // Create the login command
    let login = CreateRequest {
        email: username.clone(),
        secret: read_key,
    };

    // Attempt the login request with a 10 second timeout
    let response: Result<CreateResponse, InvokeError<CreateFailed>> = chain.invoke(login).await;
    match response {
        Err(InvokeError::Reply(CreateFailed::AlreadyExists)) => Err(CreateError::AlreadyExists),
        result => {
            let mut result = result?;

            let mut session = AteSession::default();
            session.properties.append(&mut result.authority);
            Ok(session)
        }
    }
}

pub async fn main_create(
    username: Option<String>,
    password: Option<String>,
    auth: Url
) -> Result<AteSession, CreateError>
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

    // Create a user using the authentication server which will give us a session with all the tokens
    Ok(create_command(username, password, auth).await?)
}