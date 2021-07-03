#[allow(unused_imports)]
use log::{info, error, warn, debug};
use async_trait::async_trait;
use serde::{Serialize, de::DeserializeOwned};
use std::{sync::Weak};
use std::sync::Arc;

use crate::{error::*, event::*, meta::{CoreMetadata}};
use crate::dio::*;
use crate::chain::*;
use crate::session::*;
use crate::meta::*;
use crate::header::*;

use super::*;

pub(crate) struct ServiceHook<REQ, RES, ERR>
where REQ: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized,
      RES: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized,
      ERR: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized
{
    chain: Weak<Chain>,
    session: AteSession,
    handler: ServiceInstance<REQ, RES, ERR>,
    request_type_name: String,
    response_type_name: String,
    error_type_name: String,
}

impl<REQ, RES, ERR> ServiceHook<REQ, RES, ERR>
where REQ: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized,
      RES: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized,
      ERR: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized
{
    pub(crate) fn new(chain: &Arc<Chain>, session: AteSession, handler: ServiceInstance<REQ, RES, ERR>) -> ServiceHook<REQ, RES, ERR> {
        ServiceHook {
            chain: Arc::downgrade(chain),
            session: session.clone(),
            handler: Arc::clone(&handler),
            request_type_name: std::any::type_name::<REQ>().to_string(),
            response_type_name: std::any::type_name::<RES>().to_string(),
            error_type_name: std::any::type_name::<ServiceErrorReply<ERR>>().to_string(),
        }
    }
}

#[async_trait]
impl<REQ, RES, ERR> Service
for ServiceHook<REQ, RES, ERR>
where REQ: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized,
      RES: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized,
      ERR: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized + std::fmt::Debug
{
    fn filter(&self, evt: &EventData) -> bool {
        if let Some(t) = evt.meta.get_type_name() {
            return t.type_name == self.request_type_name;
        }
        false
    }

    async fn notify(&self, key: PrimaryKey) -> Result<(), ServiceError<()>>
    {
        // Get a reference to the chain
        let chain = match self.chain.upgrade() {
            Some(a) => a,
            None => {
                return Err(ServiceError::Aborted);
            }
        };

        // Load the repository
        let repo = match chain.repository() {
            Some(a) => a,
            None => {
                warn!("service call failed - repository pointer is missing which means the service was added to a chain that is itself detached from any repositories, this is not allowed.");
                return Ok(());
            }
        };

        let ret = {
            // Load the object
            let mut dio = chain.dio(&self.session).await;
            dio.auto_cancel();
            let mut req = dio.load::<REQ>(&key).await?;

            // Attempt to lock (later delete) the request - if that fails then someone else
            // has likely picked this up and will process it instead
            if req.try_lock_then_delete(&mut dio).await? == false {
                debug!("service call skipped - someone else locked it");
                return Ok(())
            }

            // Create the context
            let context = InvocationContext
            {
                session: &self.session,
                repository: repo,
            };

            // Invoke the callback in the service
            req.commit(&mut dio)?;
            let ret = self.handler.process(req.take(), context).await;
            dio.commit().await?;
            ret
        };

        let request_type_name = std::any::type_name::<REQ>().to_string();
        match ret {
            Ok(res) => {
                debug!("service [{}] ok", request_type_name);
                self.send_reply(chain, key, res, self.response_type_name.clone()).await
            },
            Err(err) => {
                let (reply, err) = err.as_reply();
                let _ = self.send_reply(chain, key, reply, self.error_type_name.clone()).await;
                debug!("service [{}] error: {}", request_type_name, err);
                return Err(err);
            }
        }
    }
}

impl<REQ, RES, ERR> ServiceHook<REQ, RES, ERR>
where REQ: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized,
      RES: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized,
      ERR: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized
{
    async fn send_reply<T>(&self, chain: Arc<Chain>, req: PrimaryKey, res: T, res_type: String) -> Result<(), ServiceError<()>>
    where T: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized
    {
        // Turn it into a data object to be stored on commit
        let mut dio = chain.dio(&self.session).await;
        dio.auto_cancel();
        let mut res = dio.make_ext(res, self.session.log_format.clone(), None)?;

        // If the session has an encryption key then use it
        if let Some(key) = self.session.read_keys().into_iter().map(|a| a.clone()).next() {
            res.auth_mut().read = ReadOption::from_key(&key)?;
        }

        // Add the metadata
        res.add_extra_metadata(CoreMetadata::Type(MetaType {
            type_name: res_type
        }));
        res.add_extra_metadata(CoreMetadata::Reply(req));
        
        // Commit the transaction
        res.commit(&mut dio)?;
        dio.commit().await?;
        Ok(())
    }
}