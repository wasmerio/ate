#![allow(unused_imports)]
use error_chain::bail;
use std::io::Write;
use std::ops::Deref;
use std::sync::Arc;
use std::{io::stdout, path::Path};
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use url::Url;

use ate::error::LoadError;
use ate::error::TransformError;
use ate::prelude::*;
use ate::utils::chain_key_4hex;

use crate::error::*;
use crate::helper::*;
use crate::helper::*;
use crate::model::*;
use crate::prelude::*;
use crate::request::*;
use crate::service::AuthService;

impl AuthService {
    pub async fn process_gather(
        self: Arc<Self>,
        request: GatherRequest,
    ) -> Result<GatherResponse, GatherFailed> {
        debug!("gather attempt: {}", request.group);

        // Load the master key which will be used to encrypt the group so that only
        // the authentication server can access it
        let master_key = match self.master_key() {
            Some(a) => a,
            None => {
                return Err(GatherFailed::NoMasterKey);
            }
        };

        let mut super_session = AteSessionUser::default();
        super_session.user.add_read_key(&master_key);

        // Compute which chain the group should exist within
        let group_chain_key = chain_key_4hex(&request.group, Some("redo"));
        let chain = self.registry.open(&self.auth_url, &group_chain_key).await?;

        // Load the group
        let group_key = PrimaryKey::from(request.group.clone());
        let dio = chain.dio(&self.master_session).await;
        let group = match dio.load::<Group>(&group_key).await {
            Ok(a) => a,
            Err(LoadError(LoadErrorKind::NotFound(_), _)) => {
                return Err(GatherFailed::GroupNotFound(request.group));
            }
            Err(LoadError(
                LoadErrorKind::TransformationError(TransformErrorKind::MissingReadKey(_)),
                _,
            )) => {
                return Err(GatherFailed::NoMasterKey);
            }
            Err(err) => {
                bail!(err);
            }
        };

        // Now go into a loading loop on the session
        let session = complete_group_auth(group.deref(), request.session)?;

        // Return the session that can be used to access this user
        Ok(GatherResponse {
            group_name: request.group.clone(),
            gid: group.gid,
            group_key: group.key().clone(),
            authority: session,
        })
    }
}
