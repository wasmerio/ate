use ate_auth::cmd::query_command;
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
        db_url: url::Url,
        instance_authority: String,
    ) -> Result<WalletInstance, InstanceError>
    {
        // Get the sudo rights from the session (as we will use these for the wallet)
        let (sudo_read, sudo_private_read) = {
            let session = self.dio.session();
            let sudo_read = match session.read_keys(AteSessionKeyCategory::SudoKeys).next() {
                Some(a) => a,
                None => bail!(InstanceErrorKind::Unauthorized)
            };
            let sudo_private_read = match session.private_read_keys(AteSessionKeyCategory::SudoKeys).next() {
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
            (sudo_read.clone(), sudo_private_read.clone())
        };
        let all_write_keys = self.session().write_keys(AteSessionKeyCategory::AllKeys).map(|a| a.clone()).collect::<Vec<_>>();

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

        // Generate encryption keys and modify the root of the tree so that it
        // uses them
        let key_size = sudo_read.size();
        let read_key = EncryptKey::generate(key_size);
        let write_key = PrivateSignKey::generate(key_size);
        let mut chain_session = AteSessionUser::default();
        chain_session.add_user_read_key(&read_key);
        chain_session.add_user_write_key(&write_key);
        for write_key in all_write_keys {
            chain_session.add_user_write_key(&write_key);
        }
        chain_session.add_user_uid(0);
        let mut chain_session = AteSessionGroup::new(AteSessionInner::User(chain_session), self.session_identity());
        chain_session.add_group_gid(&AteRolePurpose::Observer, 0);
        chain_session.add_group_gid(&AteRolePurpose::Contributor, 0);
        chain_session.add_group_read_key(&AteRolePurpose::Observer, &read_key);
        chain_session.add_group_write_key(&AteRolePurpose::Contributor, &write_key);
        
        // Create the edge chain-of-trust
        let token = AteHash::generate();
        let key_name = format!("{}/{}_edge", self.session_identity(), token);
        let key = ChainKey::from(key_name.clone());
        let chain = self.registry.open(&db_url, &key).await?;
        let chain_api = Arc::new(
            FileAccessor::new(
                chain.as_arc(),
                group,
                AteSessionType::Group(chain_session),
                TransactionScope::Full,
                TransactionScope::Full,
                false,
                false,
            )
            .await,
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

        // Perform an authenticator query to get the edge key
        let query = query_command(&self.registry, instance_authority.clone(), self.auth.clone()).await?;
        let master_public = query.advert.broker_encrypt;

        // Output what we are encrypting with
        debug!("using instance authority ({}) public encryption key ({})", instance_authority, master_public.hash());

        // Add the master authority record so that the master servers can read this
        let admin_token = AteHash::generate().to_hex_string();
        let dio = chain_api.dio_mut_meta().await;
        let mut master_authority = dio.store_with_key(
           MasterAuthority {
               inner_broker: PublicEncryptedSecureData::new(&master_public, MasterAuthorityInner {
                   read: read_key,
                   write: write_key.clone(),
               })?,
               inner_owner: PublicEncryptedSecureData::new(sudo_private_read.as_public_key(), MasterAuthorityInner {
                read: read_key,
                write: write_key.clone(),
            })?
           },
           PrimaryKey::from(MASTER_AUTHORITY_ID),
        )?;
        master_authority.auth_mut().read = ReadOption::Everyone(None);
        master_authority.attach_orphaned(root.key())?;

        // Add the object directly to the chain        
        let mut instance_dao = dio.store_with_key(
            ServiceInstance {
                name: name.clone(),
                chain: key_name.clone(),
                admin_token,
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
        let mut wallet_instance_dao = self.dio.store_with_key(
            instance.clone(),
            instance_key,
        )?;

        // Set its permissions and attach it to the parent
        wallet_instance_dao.auth_mut().read = ReadOption::from_key(&sudo_read);
        wallet_instance_dao.auth_mut().write = WriteOption::Inherit;
        wallet_instance_dao.attach_orphaned_ext(&self.wallet.parent_id().unwrap(), INSTANCE_COLLECTION_ID)?;

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
