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
    pub async fn process_query(
        self: Arc<Self>,
        request: QueryRequest,
    ) -> Result<QueryResponse, QueryFailed> {
        debug!("query user/group: {}", request.identity);

        // Compute which chain the user should exist within
        let chain_key = chain_key_4hex(&request.identity, Some("redo"));
        let chain = self.registry.open(&self.auth_url, &chain_key, true).await?;
        let dio = chain.dio(&self.master_session).await;

        // If it does not exist then fail
        let user_key_entropy = format!("advert:{}", request.identity).to_string();
        let user_key = PrimaryKey::from(user_key_entropy);
        if dio.exists(&user_key).await == false {
            return Err(QueryFailed::NotFound);
        }

        // Load the advert
        let advert = dio.load::<Advert>(&user_key).await?;

        // Return success to the caller
        Ok(QueryResponse {
            advert: advert.take(),
        })
    }
}
