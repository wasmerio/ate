use error_chain::*;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

use crate::error::*;

use super::*;

impl DeployApi {
    pub async fn contract_get(
        &mut self,
        reference_number: &str,
    ) -> Result<ContractSummary, ContractError> {
        let contracts = self.contract_summary().await?;

        let reference_number = reference_number.trim();
        let contract = contracts
            .iter()
            .filter(|a| {
                a.reference_number.eq_ignore_ascii_case(reference_number)
                    || a.service.code.eq_ignore_ascii_case(reference_number)
            })
            .next();
        let contract = match contract {
            Some(a) => a.clone(),
            None => {
                bail!(ContractErrorKind::InvalidReference(
                    reference_number.to_string()
                ));
            }
        };
        Ok(contract)
    }
}
