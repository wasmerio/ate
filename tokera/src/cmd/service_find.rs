#[allow(unused_imports)]
use tracing::{info, error, debug, trace, warn};
use std::sync::Arc;

use ate::prelude::*;

use crate::error::*;
use crate::request::*;
use crate::model::*;

pub fn get_advertised_service<'a>(response: &'a ServiceFindResponse, service_name: &'_ str) -> Result<Option<&'a AdvertisedService>, CoreError>
{
    let service = response.services
        .iter()
        .filter(|a| a.name.to_lowercase() == service_name || a.code == service_name)
        .next();
    Ok(service)
}

pub async fn service_find_command(registry: &Arc<Registry>, service_name: Option<String>, auth: url::Url) -> Result<ServiceFindResponse, CoreError>
{
    // Open a command chain
    let chain = registry.open_cmd(&auth).await?;
    
    // Create the login command
    let query = ServiceFindRequest {
        service_name,
    };

    // Attempt the login request with a 10 second timeout
    let response: Result<ServiceFindResponse, ServiceFindFailed> = chain.invoke(query).await?;
    let result = response?;
    Ok(result)
}