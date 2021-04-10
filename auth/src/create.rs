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

        // Create the access object
        let read_key = super_key;
        let write_key = PrivateSignKey::generate(KeySize::Bit256);
        let mut access = Vec::new();
        access.push(Authorization {
            name: "loopback".to_string(),
            read: Some(read_key.clone()),
            write: Some(write_key.clone())
        });

        // Compute the super super key thats used to access the elevated rights object
        let super_super_key = match self.compute_super_key(super_key.clone()) {
            Some(a) => a,
            None => { return Err(ServiceError::Reply(CreateFailed::NoMasterKey)); }
        };

        // Create an aggregation session
        let mut session = self.master_session.clone();
        session.add_read_key(&super_key);
        session.add_read_key(&super_super_key);
        session.add_read_key(&read_key);
        session.add_write_key(&write_key);

        // Compute which chain the user should exist within
        let user_chain_key = auth_chain_key("auth".to_string(), &request.email);
        let chain = context.repository.open_by_key(&user_chain_key).await?;
        let mut dio = chain.dio(&session).await;

        // If it already exists then fail
        let user_key = PrimaryKey::from(request.email.clone());
        if dio.exists(&user_key).await {
            return Err(ServiceError::Reply(CreateFailed::AlreadyExists));
        }

        // Generate a QR code and other sudo keys
        let google_auth = google_authenticator::GoogleAuthenticator::new();
        let secret = google_auth.create_secret(32);
        let google_auth_secret = format!("otpauth://totp/{}:{}?secret={}", request.auth.to_string(), request.email, secret);
        let sudo_read_key = EncryptKey::generate(KeySize::Bit256);
        let sudo_write_key = PrivateSignKey::generate(KeySize::Bit256);
        let mut sudo_access = Vec::new();
        sudo_access.push(Authorization {
            name: "sudo".to_string(),
            read: Some(sudo_read_key.clone()),
            write: Some(sudo_write_key.clone())
        });

        // Create the user and save it
        let user = User {
            person: DaoRef::default(),
            account: DaoRef::default(),
            role: UserRole::Human,
            status: UserStatus::Nominal,
            last_login: None,
            access: access,
            foreign: DaoForeign::default(),
            sudo: DaoRef::default(),
            nominal_read: read_key.hash(),
            nominal_write: write_key.as_public_key(),
            sudo_read: sudo_read_key.hash(),
            sudo_write: sudo_write_key.as_public_key()
        };
        let mut user = Dao::make(user_key.clone(), chain.default_format(), user);
        
        // Set the authorizations amd commit the user to the tree
        user.auth_mut().read = ReadOption::Specific(read_key.hash());
        user.auth_mut().write = WriteOption::Specific(write_key.hash());

        // Build the QR image
        let qr_code = QrCode::new(google_auth_secret.as_bytes()).unwrap()
            .render::<unicode::Dense1x2>()
            .dark_color(unicode::Dense1x2::Light)
            .light_color(unicode::Dense1x2::Dark)
            .build();

        // Create the sudo object and save it using another elevation of the key
        let sudo = Sudo {
            google_auth: google_auth_secret,
            secret,
            access: sudo_access,
            qr_code: qr_code.clone(),
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
            qr_code: Some(qr_code),
            authority: session.properties,
        })
    }
}

#[allow(dead_code)]
pub async fn create_command(username: String, password: String, auth: Url) -> Result<CreateResponse, CreateError>
{
    // Open a command chain
    let chain_url = crate::helper::command_url(auth.clone());
    let registry = ate::mesh::Registry::new(&conf_auth()).await;
    let chain = registry.open_by_url(&chain_url).await?;

    // Generate a read-key using the password and some seed data
    // (this read-key will be mixed with entropy on the server side to decrypt the row
    //  which means that neither the client nor the server can get at the data alone)
    let prefix = format!("remote-login:{}:", username);
    let read_key = super::password_to_read_key(&prefix, &password, 10);
    
    // Create the login command
    let auth = match auth.domain() {
        Some(a) => a.to_string(),
        None => "ate".to_string(),
    };
    let login = CreateRequest {
        auth,
        email: username.clone(),
        secret: read_key,
    };

    // Attempt the login request with a 10 second timeout
    let response: Result<CreateResponse, InvokeError<CreateFailed>> = chain.invoke(login).await;
    match response {
        Err(InvokeError::Reply(CreateFailed::AlreadyExists)) => Err(CreateError::AlreadyExists),
        result => {
            let result = result?;
            debug!("key: {}", result.key);
            Ok(result)
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
    let mut result = create_command(username, password, auth).await?;
    println!("Account created (id={})", result.key);

    // If it has a QR code then display it
    if let Some(code) = result.qr_code {
        println!("");
        println!("Below is your Google Authenticator QR code - scan it on your phone and");
        println!("save it as this code is the only way you can recover the account.");
        println!("");
        println!("{}", code);
    }

    // Create the session
    let mut session = AteSession::default();
    session.properties.append(&mut result.authority);
    Ok(session)
}