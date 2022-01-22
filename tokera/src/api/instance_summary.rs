use error_chain::bail;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};
use std::sync::Arc;

use crate::error::*;
use crate::model::*;
use ate::prelude::*;

use super::*;

#[derive(Debug, Clone)]
pub struct InstanceSummary {
    /// Primary key of this instance
    pub key: PrimaryKey,
    /// Name of the instance
    pub name: String,
    /// Status of the instance
    pub status: InstanceStatus,
}

impl TokApi {
    pub async fn instance_summary(&mut self) -> Result<Vec<InstanceSummary>, InstanceError> {
        // Query all the instances for this wallet
        let mut ret = Vec::new();

        for instance in self.instances().await.iter().await? {
            ret.push(InstanceSummary {
                key: instance.key().clone(),
                name: instance.name.clone(),
                status: instance.status.clone(),
            })
        }

        Ok(ret)
    }

    pub async fn instances(&self) -> DaoVec<ServiceInstance>
    {
        DaoVec::<ServiceInstance>::new_orphaned_mut(
            &self.dio,
            self.wallet.parent_id().unwrap(),
            INSTANCE_COLLECTION_ID
        )
    }

    pub async fn instance_find(&self, name: &str) -> Result<DaoMut<ServiceInstance>, InstanceError>
    {
        let instance = self.instances()
            .await
            .iter_mut()
            .await?
            .filter(|i| i.name.eq_ignore_ascii_case(name))
            .next();

        let instance = match instance {
            Some(a) => a,
            None => {
                bail!(InstanceErrorKind::InvalidInstance);
            }
        };

        Ok(instance)
    }

    pub async fn instance_chain(&self, name: &str) -> Result<Arc<Chain>, InstanceError> {
        let instance = self.instance_find(name).await?;
        let instance_key = ChainKey::from(instance.chain.clone());
        let db_url: Result<_, InstanceError> = self.db_url.clone().ok_or_else(|| InstanceErrorKind::Unsupported.into());
        let chain = self.registry.open(&db_url?, &instance_key).await?;
        Ok(chain.as_arc())
    }
}
