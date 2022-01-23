use ate::chain::ChainKey;
use ate::header::PrimaryKey;
use ate::prelude::{DaoMut, DioMut};
use error_chain::bail;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};
use std::ops::Deref;
use std::sync::Arc;

use crate::error::*;
use crate::model::{INSTANCE_ROOT_ID, ServiceInstance};

use super::*;

impl TokApi {
    pub async fn instance_action(
        &mut self,
        name: &str,
    ) -> Result<(Arc<DioMut>, DaoMut<ServiceInstance>), InstanceError> {

        // If the name supplied is not good enough then fail
        let name = name.to_lowercase();
        if name.len() <= 0 {
            bail!(InstanceErrorKind::InvalidInstance);
        }
        let name = name.as_str();

        // Find the instance that best matches the name supplied
        let mut instances = self.instances().await;
        let instances = instances
            .iter_mut()
            .await?
            .filter(|i| i.name.to_lowercase().starts_with(name))
            .collect::<Vec<_>>();
        
        // If there are too many instances that match this name then fail
        if instances.len() > 1 {
            bail!(InstanceErrorKind::InvalidInstance);
        }

        // Otherwise get the instance
        let instance = instances
            .into_iter()
            .next()
            .ok_or_else(|| InstanceErrorKind::InvalidInstance)?;

        // Load the chain for the instance
        let instance_key = ChainKey::from(instance.chain.clone());
        let db_url: Result<_, InstanceError> = self.db_url.clone().ok_or_else(|| InstanceErrorKind::Unsupported.into());
        let chain = self.registry.open(&db_url?, &instance_key).await?;
        let chain_dio = chain.dio_full(self.session().deref()).await;

        // Load the instance
        let instance = chain_dio.load::<ServiceInstance>(&PrimaryKey::from(INSTANCE_ROOT_ID)).await?;
        Ok((chain_dio, instance))
    }
}
