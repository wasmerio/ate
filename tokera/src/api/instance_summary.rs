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
    /// ID of this instance within Tokera
    pub id_str: String,
    /// Chain that the instance is attached to
    pub chain: ChainKey,
}

impl TokApi {
    pub async fn instance_summary(&mut self) -> Result<Vec<InstanceSummary>, InstanceError> {
        // Query all the instances for this wallet
        let mut ret = Vec::new();

        for instance in self.instances().await.iter().await? {
            let id_str = instance.id_str();
            let chain = ChainKey::from(instance.chain.clone());
            ret.push(InstanceSummary {
                key: instance.key().clone(),
                name: instance.name.clone(),
                chain,
                id_str,
            })
        }

        Ok(ret)
    }

    pub async fn instances(&self) -> DaoVec<WalletInstance>
    {
        DaoVec::<WalletInstance>::new_orphaned_mut(
            &self.dio,
            self.wallet.parent_id().unwrap(),
            INSTANCE_COLLECTION_ID
        )
    }

    pub async fn instance_find_exact(&self, name: &str) -> Result<DaoMut<WalletInstance>, InstanceError>
    {
        let instance = self.instances()
            .await
            .iter_mut_ext(true, true)
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

    pub async fn instance_find(&self, name: &str) -> Result<DaoMut<WalletInstance>, InstanceError>
    {
        // If the name supplied is not good enough then fail
        let name = name.to_lowercase();
        if name.len() <= 0 {
            bail!(InstanceErrorKind::InvalidInstance);
        }
        let name = name.as_str();

        // Find the instance that best matches the name supplied
        let mut instances = self.instances().await;
        let instances = instances
            .iter_mut_ext(true, true)
            .await?
            .filter(|i| i.name.to_lowercase().starts_with(name))
            .collect::<Vec<_>>();
        
        // If there are too many instances that match this name then fail
        if instances.len() > 1 {
            bail!(InstanceErrorKind::InvalidInstance);
        }
        let instance = instances.into_iter().next();

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
