#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};
use error_chain::bail;
use ate_files::accessor::FileAccessor;
use ate::crypto::AteHash;
use ate::chain::ChainKey;
use std::sync::Arc;
use ate::prelude::*;

use crate::error::*;
use crate::model::*;

use super::*;

impl TokApi {
    pub async fn instance_create(
        &mut self,
        name: String,
        group: Option<String>,
        session: AteSessionType,
        db_url: url::Url,
    ) -> Result<WalletInstance, InstanceError>
    {
        // Get the sudo rights from the session (as we will use these for the wallet)
        let sudo_read = {
            let session = self.dio.session();
            let sudo_read = match session.read_keys(AteSessionKeyCategory::SudoKeys).next() {
                Some(a) => a,
                None => bail!(InstanceErrorKind::Unauthorized)
            };
            if session
                .write_keys(AteSessionKeyCategory::SudoKeys)
                .next()
                .is_none()
            {
                bail!(InstanceErrorKind::Unauthorized);
            };
            sudo_read.clone()
        };

        // If it already exists then fail
        let instance_key_entropy = format!("instance://{}/{}", self.session_identity(), name);
        let instance_key = PrimaryKey::from(instance_key_entropy);
        if self.dio.exists(&instance_key).await {
            bail!(InstanceErrorKind::AlreadyExists);
        }

        // Check if the instance already exists
        let instances = self.instances().await;
        if instances.iter().await?.any(|i| i.name.eq_ignore_ascii_case(name.as_str())) {
            bail!(InstanceErrorKind::AlreadyExists);
        }
        
        // Create the edge chain-of-trust
        let token = AteHash::generate();
        let key_name = format!("{}/{}_edge", self.session_identity(), token);
        let key = ChainKey::from(key_name.clone());
        let chain = self.registry.open(&db_url, &key).await?;
        let chain_api = Arc::new(
            FileAccessor::new(
                chain.as_arc(),
                group,
                session,
                TransactionScope::Full,
                TransactionScope::Full,
                false,
                false,
            )
            .await
            .with_force_sudo(true),
        );

        // Initialize and save the chain_api
        debug!("intiializing chain-of-trust: {}", key);
        let root = chain_api.init(&chain_api.session_context()).await?;
        for dir in vec![ "bin", "dev", "etc", "tmp" ] {
            if chain_api.search(&chain_api.session_context(), format!("/{}", dir).as_str()).await?.is_none() {
                debug!("creating directory: {}", dir);
                chain_api.mkdir(&chain_api.session_context(), root.key().as_u64(), dir, root.dentry.mode).await?;
            }
        }

        // Add the object directly to the chain
        let dio = chain_api.dio_mut_meta().await;
        let mut instance_dao = dio.store_with_key(
            ServiceInstance {
                name: name.clone(),
                chain: key_name.clone(),
                exports: DaoVec::new(),
            },
            PrimaryKey::from(INSTANCE_ROOT_ID),
        )?;
        instance_dao.attach_orphaned(root.key())?;
        chain_api.commit().await?;
        dio.commit().await?;

        // Create the instance and add it to the identities collection
        debug!("adding service instance: {}", name);
        let instance = WalletInstance {
            name: name.clone(),
            chain: key_name,
        };
        let mut instance_dao = self.dio.store_with_key(
            instance.clone(),
            instance_key,
        )?;

        // Set its permissions and attach it to the parent
        instance_dao.auth_mut().read = ReadOption::from_key(&sudo_read);
        instance_dao.auth_mut().write = WriteOption::Inherit;
        instance_dao.attach_orphaned_ext(&self.wallet.parent_id().unwrap(), INSTANCE_COLLECTION_ID)?;

        // Now add the history
        if let Err(err) = self
            .record_activity(HistoricActivity::InstanceCreated(
                activities::InstanceCreated {
                    when: chrono::offset::Utc::now(),
                    by: self.user_identity(),
                    alias: Some(name),
                },
            ))
            .await
        {
            error!("Error writing activity: {}", err);
        }
        self.dio.commit().await?;

        Ok(instance)
    }
}
