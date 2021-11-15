use chrono::DateTime;
use chrono::Utc;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

use crate::error::*;
use crate::model::*;
use ate::prelude::*;

use super::*;

#[derive(Debug, Clone)]
pub struct ContractSummary {
    /// Primary key of this contract
    pub key: PrimaryKey,
    /// Reference number assocaited with this contract
    pub reference_number: String,
    /// The advertised service being consumed by the provider
    pub service: AdvertisedService,
    /// Status of the contract
    pub status: ContractStatus,
    /// Limited duration contracts will expire after a
    /// certain period of time without incurring further
    /// charges
    pub expires: Option<DateTime<Utc>>,
    /// Metrics currently tracked for this contract
    pub metrics: Vec<ContractMetrics>,
}

impl TokApi {
    pub async fn contract_summary(&mut self) -> Result<Vec<ContractSummary>, ContractError> {
        // Query all the contracts for this wallet
        let mut ret = Vec::new();

        if let Some(parent_id) = self.wallet.parent_id() {
            let contracts = self
                .dio
                .children_ext::<Contract>(parent_id, CONTRACT_COLLECTION_ID, true, true)
                .await?;
            for contract in contracts {
                let metrics = contract
                    .metrics
                    .iter()
                    .await?
                    .map(|a| a.take())
                    .collect::<Vec<_>>();

                ret.push(ContractSummary {
                    key: contract.key().clone(),
                    reference_number: contract.reference_number.clone(),
                    service: contract.service.clone(),
                    status: contract.status.clone(),
                    expires: contract.expires,
                    metrics,
                })
            }
        }

        Ok(ret)
    }
}
