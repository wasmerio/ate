#![allow(unused_imports)]
use log::{info, error, debug};
use async_trait::async_trait;
use std::sync::Arc;

use ate::prelude::*;
use ate::error::*;

use crate::commands::*;
use crate::helper::*;
use crate::model::*;

#[derive(Debug)]
struct AuthService
{
    master_session: AteSession
}

#[async_trait]
impl ServiceHandler<LoginRequest, LoginResponse, LoginFailed>
for AuthService
{
    async fn process<'a>(&self, request: LoginRequest, context: InvocationContext<'a>) -> Result<LoginResponse, ServiceError<LoginFailed>>
    {
        info!("login attempt: {}", request.email);
        
        // Create a session with crypto keys based off the username and password
        let master_key = match self.master_session.read_keys().into_iter().next() {
            Some(a) => a.clone(),
            None => {
                return Err(ServiceError::Reply(LoginFailed::NoMasterKey));
            }
        };
        let super_key = AteHash::from_bytes_twice(master_key.value(), request.secret.value());
        let super_key = EncryptKey::from_seed_bytes(super_key.to_bytes(), KeySize::Bit256);
        let mut session = AteSession::default();
        session.add_read_key(&super_key);

        // Compute which chain the user should exist within
        let user_chain_key = auth_chain_key("auth".to_string(), &request.email);
        let chain = context.repository.open_by_key(&user_chain_key).await?;
        let mut dio = chain.dio(&session).await;

        // Attempt to load the object (if it fails we will tell the caller)
        let user_key = PrimaryKey::from(request.email.clone());
        let user = match dio.load::<User>(&user_key).await {
            Ok(a) => a,
            Err(LoadError::NotFound(_)) => {
                return Err(ServiceError::Reply(LoginFailed::NotFound));
            },
            Err(err) => {
                return Err(ServiceError::LoadError(err));
            }
        };
        
        // Check if the account is locked or not yet verified
        match user.status {
            UserStatus::Locked => {
                return Err(ServiceError::Reply(LoginFailed::AccountLocked));
            },
            UserStatus::Unverified => {
                return Err(ServiceError::Reply(LoginFailed::Unverified));
            },
            UserStatus::Nominal => { },
        };

        // Add all the authorizations
        let mut session = session.clone();
        for auth in user.access.iter() {
            if let Some(read) = &auth.read {
                session.add_read_key(read);
            }
            if let Some(write) = &auth.write {
                session.add_write_key(write);
            }
        }

        // Return the session that can be used to access this user
        Ok(LoginResponse {
            authority: session.properties.clone()
        })
    }   
}

pub async fn service_logins(cmd_session: AteSession, auth_session: AteSession, chain: &Arc<Chain>)
{
    chain.add_service(cmd_session.clone(), Arc::new(
        AuthService
        {
            master_session: auth_session,
        }
    ));
}