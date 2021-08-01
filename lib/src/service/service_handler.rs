#[allow(unused_imports)]
use tracing::{info, error, warn, debug};
use async_trait::async_trait;
use serde::{Serialize, de::DeserializeOwned};

use crate::{error::*};

use super::*;

#[async_trait]
pub trait ServiceHandler<REQ, RES, ERR>
where REQ: Serialize + DeserializeOwned + Sync + Send + ?Sized,
      RES: Serialize + DeserializeOwned + Sync + Send + ?Sized,
      ERR: Serialize + DeserializeOwned + Sync + Send + ?Sized
{
    async fn process<'a>(&self, request: REQ, context: InvocationContext<'a>) -> Result<RES, ServiceError<ERR>>;
}