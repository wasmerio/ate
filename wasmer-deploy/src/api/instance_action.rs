use ate::header::PrimaryKey;
use ate::prelude::*;
use error_chain::bail;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};
use std::ops::Deref;

use crate::error::*;
use crate::model::{INSTANCE_ROOT_ID, ServiceInstance, WalletInstance, MasterAuthority, MASTER_AUTHORITY_ID};

use super::*;

impl DeployApi {
    pub async fn instance_load(&self, wallet_instance: &WalletInstance) -> Result<DaoMut<ServiceInstance>, LoadError>
    {
        // Get the sudo rights from the session (as we will use these for the wallet)
        let sudo_private_read = match self.session().private_read_keys(AteSessionKeyCategory::SudoKeys).next() {
            Some(a) => a.clone(),
            None => bail!(LoadErrorKind::IO("the current session does not have private read key.".to_string()))
        };

        // First we load the chain which we need to get the master authority object
        let instance_key = ChainKey::from(wallet_instance.chain.clone());
        let db_url: Result<_, LoadError> = self.db_url.clone().ok_or_else(|| LoadErrorKind::IO("the db_url is not set which is required to access instances".to_string()).into());
        let chain = self.registry.open(&db_url?, &instance_key).await?;
        let dio = chain.dio(self.session().deref()).await;
        
        // Now we read the chain of trust and attempt to get the master authority object
        let master_authority = dio.load::<MasterAuthority>(&PrimaryKey::from(MASTER_AUTHORITY_ID)).await?;
        let master_authority = master_authority.inner_owner.unwrap(&sudo_private_read)?;

        // Build the session using the master authority
        let mut chain_session = AteSessionUser::default();
        chain_session.add_user_read_key(&master_authority.read);
        chain_session.add_user_write_key(&master_authority.write);
        chain_session.add_user_uid(0);
        let mut chain_session = AteSessionGroup::new(AteSessionInner::User(chain_session), self.session().identity().to_string());
        chain_session.add_group_gid(&AteRolePurpose::Observer, 0);
        chain_session.add_group_gid(&AteRolePurpose::Contributor, 0);
        chain_session.add_group_read_key(&AteRolePurpose::Observer, &master_authority.read);
        chain_session.add_group_write_key(&AteRolePurpose::Contributor, &master_authority.write);
        
        // Load the instance
        let chain_dio = chain.dio_full(&chain_session).await;
        chain_dio.load::<ServiceInstance>(&PrimaryKey::from(INSTANCE_ROOT_ID)).await
    }

    pub async fn instance_action(
        &mut self,
        name: &str,
    ) -> Result<(Result<DaoMut<ServiceInstance>, LoadError>, DaoMut<WalletInstance>), InstanceError> {

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

        // Otherwise get the instance
        let wallet_instance = instances
            .into_iter()
            .next()
            .ok_or_else(|| InstanceErrorKind::InvalidInstance)?;

        
        let service_instance = self.instance_load(wallet_instance.deref()).await;
        Ok((service_instance, wallet_instance))
    }
}
