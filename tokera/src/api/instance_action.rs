use std::ops::Deref;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

use crate::cmd::*;
use crate::error::*;
use crate::request::*;

use super::*;

impl TokApi {
    pub async fn instance_action(
        &mut self,
        token: &str,
        requester_identity: &str,
        consumer_identity: &str,
        action: InstanceAction,
    ) -> Result<InstanceActionResponse, InstanceError> {
        // Execute the action
        let session = self.dio.session().clone_session();
        let ret = instance_action_command(
            &self.registry,
            session.deref(),
            self.auth.clone(),
            token.to_string(),
            requester_identity.to_string(),
            consumer_identity.to_string(),
            action,
        )
        .await?;
        Ok(ret)
    }
}
