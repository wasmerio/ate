#[allow(unused_imports)]
use tracing::{info, error, warn, debug};
use async_trait::async_trait;
use serde::{Serialize, de::DeserializeOwned};
use std::sync::Arc;

use crate::error::*;
use crate::chain::Chain;
use crate::session::AteSession;
use crate::service::ServiceHook;

use super::*;

#[async_trait]
pub trait ServiceHandler<REQ, RES, ERR>
where REQ: Serialize + DeserializeOwned + Sync + Send + ?Sized,
      RES: Serialize + DeserializeOwned + Sync + Send + ?Sized,
      ERR: Serialize + DeserializeOwned + Sync + Send + ?Sized
{
    async fn process<'a>(&self, request: REQ, context: InvocationContext<'a>) -> Result<RES, ServiceError<ERR>>;
}

impl Chain
{
    pub fn add_service<'a, 'b, REQ, RES, ERR>(self: &'a Arc<Self>, session: AteSession, handler: ServiceInstance<REQ, RES, ERR>)
    -> Arc<ServiceHook<REQ, RES, ERR>>
    where REQ: Serialize + DeserializeOwned + Sync + Send + ?Sized + 'static,
          RES: Serialize + DeserializeOwned + Sync + Send + ?Sized + 'static,
          ERR: std::fmt::Debug + Serialize + DeserializeOwned + Sync + Send + ?Sized + 'static,
    {
        let ret = Arc::new(ServiceHook::new(
            self,
            session,
            Arc::clone(&handler),
        ));

        {
            let svr = Arc::clone(&ret);
            let svr: Arc<dyn Service> = svr;
            let mut guard = self.inside_sync.write();
            guard.services.push(svr);
        }
        ret
    }
}