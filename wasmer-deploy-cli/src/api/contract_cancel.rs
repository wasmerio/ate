use error_chain::*;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

use crate::error::*;
use crate::request::*;

use super::*;

impl DeployApi {
    pub async fn contract_cancel(
        &mut self,
        reference_number: &str,
        consumer_identity: &str,
    ) -> Result<ContractActionResponse, ContractError> {
        let contracts = self.contract_summary().await?;

        // Grab the contract this action is for
        let reference_number = reference_number.trim();
        let contract = contracts
            .iter()
            .filter(|a| a.reference_number.eq_ignore_ascii_case(reference_number))
            .next();
        let contract = match contract {
            Some(a) => a.clone(),
            None => {
                bail!(ContractErrorKind::InvalidReference(
                    reference_number.to_string()
                ));
            }
        };

        let ret = self
            .contract_action(
                &contract.service.code,
                consumer_identity,
                consumer_identity,
                ContractAction::Cancel,
                None,
            )
            .await?;
        Ok(ret)
    }
}
