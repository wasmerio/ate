#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use error_chain::bail;
use std::io::stdout;
use std::io::Write;
use url::Url;
use std::ops::Deref;
use qrcode::QrCode;
use qrcode::render::unicode;
use std::sync::Arc;

use ate::prelude::*;
use ate::error::LoadError;
use ate::utils::chain_key_4hex;

use crate::prelude::*;
use crate::request::*;
use crate::service::AuthService;
use crate::helper::*;
use crate::error::*;
use crate::helper::*;

impl AuthService
{
    pub async fn process_query(self: Arc<Self>, request: QueryRequest) -> Result<QueryResponse, QueryFailed>
    {
        info!("query user/group: {}", request.identity);

        // Compute which chain the user should exist within
        let chain_key = chain_key_4hex(&request.identity, Some("redo"));
        let chain = self.registry.open(&self.auth_url, &chain_key).await?;
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