use std::ops::Deref;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

use crate::cmd::*;
use crate::error::*;
use crate::model::*;
use crate::request::*;
use ate::prelude::*;

use super::*;

impl TokApi {
    pub async fn instance_create(
        &mut self,
        wapm: String,
        stateful: bool,
        owner_identity: String,
    ) -> Result<InstanceCreateResponse, InstanceError> {
        // Make the session
        let session = self.dio.session().clone_session();

        // Create the instance
        let ret = instance_create_command(
            &self.registry,
            session.deref(),
            self.auth.clone(),
            wapm.clone(),
            stateful,
            owner_identity,
            self.wallet.key().clone(),
        )
        .await?;

        // Now add the history
        if let Err(err) = self
            .record_activity(HistoricActivity::InstanceCreated(
                activities::InstanceCreated {
                    when: chrono::offset::Utc::now(),
                    by: self.user_identity(),
                    wapm,
                    stateful
                },
            ))
            .await
        {
            error!("Error writing activity: {}", err);
        }

        Ok(ret)
    }
}
