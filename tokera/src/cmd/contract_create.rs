#[allow(unused_imports)]
use tracing::{info, error, debug, trace, warn};
use std::sync::Arc;
use url::Url;

use ate::prelude::*;

use crate::error::*;
use crate::model::*;
use crate::request::*;
use crate::helper::*;

pub async fn contract_create_command(
    registry: &Arc<Registry>,
    session: &dyn AteSession,
    auth: Url,
    service_code: String,
    identity: String,
    gst_country: Country,
    consumer_wallet: PrimaryKey,
    broker_key: PublicEncryptedSecureData<EncryptKey>,
    broker_unlock_key: EncryptKey,
) -> Result<ContractCreateResponse, ContractError>
{
    // Open a command chain
    let chain = registry.open_cmd(&auth).await?;

    // Now contract the request to create the contract
    let sign_key = session_sign_key(session, identity.contains("@"))?;
    let contract_create = ContractCreateRequest {
        consumer_identity: identity,
        params: SignedProtectedData::new(sign_key, ContractCreateRequestParams {
            service_code,
            gst_country,
            consumer_wallet,
            broker_unlock_key,
            broker_key,
            limited_duration: None,
        })?,
    };

    // Attempt the create contract request
    let response: Result<ContractCreateResponse, ContractCreateFailed> = chain.invoke(contract_create).await?;
    let result = response?;
    Ok(result)
}