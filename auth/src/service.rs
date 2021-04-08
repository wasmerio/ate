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
        // Create a session with crypto keys based off the username and password
        let master_key = self.master_session.read_keys().into_iter().next().expect("The authentication service must be loaded with a master encryption key.");
        let super_key = EncryptKey::xor(request.secret.clone(), master_key.clone())?;
        let mut session = AteSession::default();
        session.add_read_key(&super_key);

        // Compute which chain the user should exist within
        let user_chain_key = auth_chain_key("auth".to_string(), &request.email);
        let chain = context.repository.open_by_key(&user_chain_key).await?;
        let mut dio = chain.dio(&session).await;

        // Attempt to load the object (if it fails we will tell the caller)
        info!("login attempt: {}", request.email);
        let user_key = PrimaryKey::from(request.email.clone());
        let _user = match dio.load::<User>(&user_key).await {
            Ok(a) => a,
            Err(LoadError::NotFound(_)) => {
                return Err(ServiceError::Reply(LoginFailed::NotFound));
            },
            Err(err) => {
                return Err(ServiceError::LoadError(err));
            }
        };
        

        Err(ServiceError::Reply(LoginFailed::AccountLocked))
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