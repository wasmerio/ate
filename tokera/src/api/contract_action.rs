#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace};
use std::ops::Deref;

use ate::prelude::*;
use crate::error::*;
use crate::cmd::*;
use crate::request::*;

use super::*;

impl TokApi
{
    pub async fn contract_elevate(&mut self, service_code: &str, requester_identity: &str, consumer_identity: &str) -> Result<EncryptKey, ContractError>
    {
        // Execute the action
        let session = self.dio.session().clone_session();
        let ret = contract_elevate_command(
                &self.registry,
                session.deref(),
                self.auth.clone(),
                service_code.to_string(),
                requester_identity.to_string(),
                consumer_identity.to_string()
            ).await?;
        Ok(ret)
    }

    pub async fn contract_action(&mut self, service_code: &str, requester_identity: &str, consumer_identity: &str, action: ContractAction, action_key: Option<EncryptKey>) -> Result<ContractActionResponse, ContractError>
    {
        // Execute the action
        let session = self.dio.session().clone_session();
        let ret = contract_action_command(
                &self.registry,
                session.deref(),
                self.auth.clone(),
                service_code.to_string(),
                requester_identity.to_string(),
                consumer_identity.to_string(),
                action_key,
                action
            ).await?;
        Ok(ret)
    }
}