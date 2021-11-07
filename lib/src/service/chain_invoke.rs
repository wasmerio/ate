#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use error_chain::bail;
use serde::{Serialize, de::DeserializeOwned};
use std::{time::Duration};
use tokio::select;
use std::sync::Arc;
#[allow(unused_imports)]
use std::ops::Deref;

use crate::transaction::TransactionScope;
use crate::{error::*, meta::{CoreMetadata}};
use crate::dio::*;
use crate::chain::*;
use crate::session::*;
use crate::meta::*;
use crate::engine::*;

use super::*;

impl Chain
{
    pub async fn invoke<REQ, RES, ERR>(self: Arc<Self>, request: REQ) -> Result<Result<RES, ERR>, InvokeError>
    where REQ: Clone + Serialize + DeserializeOwned + Sync + Send + ?Sized,
          RES: Serialize + DeserializeOwned + Sync + Send + ?Sized,
          ERR: Serialize + DeserializeOwned + Sync + Send + ?Sized,
    {
        self.invoke_ext(None, request, std::time::Duration::from_secs(30)).await
    }

    pub async fn invoke_ext<REQ, RES, ERR>(self: Arc<Self>, session: Option<&'_ dyn AteSession>, request: REQ, timeout: Duration) -> Result<Result<RES, ERR>, InvokeError>
    where REQ: Clone + Serialize + DeserializeOwned + Sync + Send + ?Sized,
          RES: Serialize + DeserializeOwned + Sync + Send + ?Sized,
          ERR: Serialize + DeserializeOwned + Sync + Send + ?Sized,
    {
        TaskEngine::run_until(self.__invoke_ext(session, request, timeout)).await
    }

    pub(crate) async fn __invoke_ext<REQ, RES, ERR>(self: Arc<Self>, session: Option<&'_ dyn AteSession>, request: REQ, timeout: Duration) -> Result<Result<RES, ERR>, InvokeError>
    where REQ: Clone + Serialize + DeserializeOwned + Sync + Send + ?Sized,
          RES: Serialize + DeserializeOwned + Sync + Send + ?Sized,
          ERR: Serialize + DeserializeOwned + Sync + Send + ?Sized,
    {
        // If no session was provided then use the empty one
        let session_store;
        let session = match session {
            Some(a) => a,
            None => {
                session_store = self.inside_sync.read().unwrap().default_session.clone_session();
                session_store.deref()
            }
        };

        // Build the command object
        let dio = self.__dio_trans(session, TransactionScope::None).await;
        let (join_res, join_err) = {
            dio.auto_cancel();
            
            let mut cmd = dio.store(request)?;
            
            // Add an encryption key on the command (if the session has one)
            if let Some(key) = session.read_keys(AteSessionKeyCategory::AllKeys).into_iter().next() {
                cmd.auth_mut().read = ReadOption::from_key(key);
            }
            if session.write_keys(AteSessionKeyCategory::AllKeys).next().is_none() {
                cmd.auth_mut().write = WriteOption::Everyone;
            }

            // Add the extra metadata about the type so the other side can find it
            cmd.add_extra_metadata(CoreMetadata::Type(MetaType {
                type_name: std::any::type_name::<REQ>().to_string()
            }))?;

            // Sniff out the response object
            let cmd_id = cmd.key().clone();

            let response_type_name = std::any::type_name::<RES>().to_string();
            let error_type_name = std::any::type_name::<ERR>().to_string();

            let sniff_res = sniff_for_command_begin(Arc::downgrade(&self), Box::new(move |h| {
                if let Some(reply) = h.meta.is_reply_to_what() {
                    if reply == cmd_id {
                        if let Some(t) = h.meta.get_type_name() {
                            return t.type_name == response_type_name;
                        }
                    }
                }
                false
            }));
            let sniff_err = sniff_for_command_begin(Arc::downgrade(&self), Box::new(move |h| {
                if let Some(reply) = h.meta.is_reply_to_what() {
                    if reply == cmd_id {
                        if let Some(t) = h.meta.get_type_name() {
                            return t.type_name == error_type_name;
                        }
                    }
                }
                false
            }));

            // Send our command
            dio.commit().await?;

            // Wait for the response
            let join_res = sniff_for_command_finish(sniff_res);
            let join_err = sniff_for_command_finish(sniff_err);
            (join_res, join_err)
        };

        // The caller will wait on the response from the sniff that is looking for a reply object
        let mut timeout = tokio::time::interval(timeout);
        timeout.tick().await;
        select! {
            key = join_res => {
                let key = match key {
                    Some(a) => a,
                    None => { bail!(InvokeErrorKind::Aborted); }
                };
                let ret = dio.load_and_take::<RES>(&key).await?;
                if dio.delete(&key).await.is_ok() {
                    let _ = dio.commit().await;
                }
                Ok(Ok(ret))
            },
            key = join_err => {
                let key = match key {
                    Some(a) => a,
                    None => { bail!(InvokeErrorKind::Aborted); }
                };
                let ret = dio.load_and_take::<ERR>(&key).await?;
                if dio.delete(&key).await.is_ok() {
                    let _ = dio.commit().await;
                }
                Ok(Err(ret))
            },
            _ = timeout.tick() => {
                Err(InvokeErrorKind::Timeout.into())
            }
        }  
    }
}