#![allow(unused_imports)]
use error_chain::bail;
use qrcode::render::unicode;
use qrcode::QrCode;
use std::io::stdout;
use std::io::Write;
use std::ops::Deref;
use std::sync::Arc;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use url::Url;

use ate::error::LoadError;
use ate::error::TransformError;
use ate::prelude::*;
use ate::session::AteRolePurpose;
use ate::utils::chain_key_4hex;

use crate::error::*;
use crate::helper::*;
use crate::model::*;
use crate::prelude::*;
use crate::request::*;
use crate::service::AuthService;

impl AuthService {
    pub async fn process_group_remove(
        self: Arc<Self>,
        request: GroupRemoveRequest,
    ) -> Result<GroupRemoveResponse, GroupRemoveFailed> {
        info!("group ({}) remove", request.group);

        // Copy the request session
        let request_session = request.session;

        // Compute which chain the group should exist within
        let group_chain_key = chain_key_4hex(&request.group, Some("redo"));
        let chain = self.registry.open(&self.auth_url, &group_chain_key, true).await?;

        // Create the super session that has all the rights we need
        let mut super_session = self.master_session.clone();
        super_session.append(request_session.properties());

        // Delete the group
        let group_key = PrimaryKey::from(request.group.clone());
        let dio = chain.dio_full(&super_session).await;
        dio.delete(&group_key).await?;

        // Delete the advert
        let advert_key_entropy = format!("advert:{}", request.group.clone()).to_string();
        let advert_key = PrimaryKey::from(advert_key_entropy);
        let _ = dio.delete(&advert_key).await;

        // Commit
        dio.commit().await?;

        // Return success to the caller
        Ok(GroupRemoveResponse { key: group_key })
    }
}
