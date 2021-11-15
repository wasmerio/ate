use error_chain::*;
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
    pub async fn contract_create(
        &mut self,
        service: AdvertisedService,
    ) -> Result<ContractCreateResponse, ContractError> {
        // Make the session
        let session = self.dio.session().clone_session();

        // Grab the latest encryption key for the provider (this will be used to encrypt
        // the broker key)
        let advert = {
            match query_command(&self.registry, service.owner_identity.clone(), self.auth.clone()).await {
                Ok(a) => a,
                Err(err) => {
                    let err_code = ate::utils::obscure_error_str(format!("Failed to create the contract as the query to the authentication server failed - {}.", err.to_string()).as_str());
                    bail!(ContractErrorKind::CoreError(CoreErrorKind::InternalError(err_code)));
                }
            }.advert
        };
        let broker_key =
            PublicEncryptedSecureData::new(&advert.broker_encrypt, self.wallet.broker_key.clone())?;

        // Create the contract
        let gst_country = self.wallet.gst_country;
        let ret = contract_create_command(
            &self.registry,
            session.deref(),
            self.auth.clone(),
            service.code.clone(),
            self.session_identity(),
            gst_country,
            self.wallet.key().clone(),
            broker_key,
            self.wallet.broker_unlock_key.clone(),
        )
        .await?;

        // Now add the history
        if let Err(err) = self
            .record_activity(HistoricActivity::ContractCreated(
                activities::ContractCreated {
                    when: chrono::offset::Utc::now(),
                    by: self.user_identity(),
                    service: service.clone(),
                    contract_reference: ret.contract_reference.clone(),
                },
            ))
            .await
        {
            error!("Error writing activity: {}", err);
        }

        Ok(ret)
    }
}
