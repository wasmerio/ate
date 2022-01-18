#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

use crate::error::*;
use crate::model::*;
use ate::prelude::*;

use super::*;

#[derive(Debug, Clone)]
pub struct InstanceSummary {
    /// Primary key of this instance
    pub key: PrimaryKey,
    /// Token associated with the instance
    pub token: String,
    /// Status of the instance
    pub status: InstanceStatus,
    /// Reference to the contract associated with this instance
    pub contract: Option<ContractSummary>,
}

impl TokApi {
    pub async fn instance_summary(&mut self) -> Result<Vec<InstanceSummary>, InstanceError> {
        // Query all the instances for this wallet
        let mut ret = Vec::new();

        if let Some(parent_id) = self.wallet.parent_id() {
            let instances = self
                .dio
                .children_ext::<ServiceInstance>(parent_id, INSTANCE_COLLECTION_ID, true, true)
                .await?;
            for instance in instances {
                let contract = instance.contract
                    .load().await?;

                let contract = match contract {
                    Some(c) => Some(self.get_contract_summary(&c).await?),
                    None => None
                };

                ret.push(InstanceSummary {
                    key: instance.key().clone(),
                    token: instance.token.clone(),
                    status: instance.status.clone(),
                    contract,
                })
            }
        }

        Ok(ret)
    }
}
