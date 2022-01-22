use ate::chain::ChainKey;
use error_chain::bail;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};
use std::ops::Deref;

use crate::error::*;
use crate::model::{activities, HistoricActivity};
use crate::request::*;

use super::*;

impl TokApi {
    pub async fn instance_action(
        &mut self,
        name: &str,
        action: InstanceAction,
    ) -> Result<(), InstanceError> {

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
        if instances.len() <= 0 {
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

        // Based on the action we are going to do something
        match action
        {
            InstanceAction::Kill => {
                let wapm = instance.wapm.clone();
                let name = instance.name.clone();

                debug!("deleting all the roots in the chain");
                chain_dio.delete_all_roots().await?;
                chain_dio.commit().await?;
                drop(chain_dio);
                drop(chain);

                // Now add the history
                if let Err(err) = self
                    .record_activity(HistoricActivity::InstanceDestroyed(
                        activities::InstanceDestroyed {
                            when: chrono::offset::Utc::now(),
                            by: self.user_identity(),
                            wapm: wapm.clone(),
                            alias: Some(name.clone()),
                        },
                    ))
                    .await
                {
                    error!("Error writing activity: {}", err);
                }

                debug!("deleting the instance from the user/group");
                let _ = instance.delete()?;
                self.dio.commit().await?;

                println!("Instance ({} with alias {}) has been killed", wapm, name);
                Ok(())
            }
            _ => {
                bail!(InstanceErrorKind::Unsupported);
            }
        }
    }
}
