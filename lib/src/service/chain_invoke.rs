#[allow(unused_imports)]
use tracing::{info, error, warn, debug};
use serde::{Serialize, de::DeserializeOwned};
use std::{time::Duration};
use tokio::select;
use std::sync::Arc;

use crate::{error::*, meta::{CoreMetadata}};
use crate::dio::*;
use crate::chain::*;
use crate::session::*;
use crate::meta::*;

use super::*;

impl Chain
{
    pub async fn invoke<REQ, RES, ERR>(self: Arc<Self>, request: REQ) -> Result<RES, InvokeError<ERR>>
    where REQ: Serialize + DeserializeOwned + Sync + Send + ?Sized,
          RES: Serialize + DeserializeOwned + Sync + Send + ?Sized,
          ERR: Serialize + DeserializeOwned + Sync + Send + ?Sized,
    {
        self.invoke_ext(None, request, std::time::Duration::from_secs(30)).await
    }

    pub async fn invoke_ext<REQ, RES, ERR>(self: Arc<Self>, session: Option<&AteSession>, request: REQ, timeout: Duration) -> Result<RES, InvokeError<ERR>>
    where REQ: Serialize + DeserializeOwned + Sync + Send + ?Sized,
          RES: Serialize + DeserializeOwned + Sync + Send + ?Sized,
          ERR: Serialize + DeserializeOwned + Sync + Send + ?Sized,
    {
        // If no session was provided then use the empty one
        let session_store;
        let session = match session {
            Some(a) => a,
            None => {
                session_store = self.inside_sync.read().default_session.clone();
                &session_store
            }
        };

        // Build the command object
        let dio = self.dio_forget(session).await;
        let (join_res, join_err) = {
            dio.auto_cancel();
            
            let mut cmd = dio.store(request)?;
            
            // Add an encryption key on the command (if the session has one)
            if let Some(key) = session.read_keys().into_iter().next() {
                cmd.auth_mut().read = ReadOption::from_key(key);
            }

            // Add the extra metadata about the type so the other side can find it
            cmd.add_extra_metadata(CoreMetadata::Type(MetaType {
                type_name: std::any::type_name::<REQ>().to_string()
            }))?;

            // Sniff out the response object
            let cmd_id = cmd.key().clone();

            let response_type_name = std::any::type_name::<RES>().to_string();
            let error_type_name = std::any::type_name::<ServiceErrorReply<ERR>>().to_string();

            let join_res = sniff_for_command(Arc::downgrade(&self), Box::new(move |h| {
                if let Some(reply) = h.meta.is_reply_to_what() {
                    if reply == cmd_id {
                        if let Some(t) = h.meta.get_type_name() {
                            return t.type_name == response_type_name;
                        }
                    }
                }
                false
            }));
            let join_err = sniff_for_command(Arc::downgrade(&self), Box::new(move |h| {
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

            (join_res, join_err)
        };

        // The caller will wait on the response from the sniff that is looking for a reply object
        let mut timeout = tokio::time::interval(timeout);
        timeout.tick().await;
        select! {
            key = join_res => {
                let key = match key {
                    Some(a) => a,
                    None => { return Err(InvokeError::Aborted); }
                };
                Ok(dio.load_and_take::<RES>(&key).await?)
            },
            key = join_err => {
                let key = match key {
                    Some(a) => a,
                    None => { return Err(InvokeError::Aborted); }
                };
                match dio.load_and_take::<ServiceErrorReply<ERR>>(&key).await? {
                    ServiceErrorReply::Reply(e) => Err(InvokeError::Reply(e)),
                    ServiceErrorReply::ServiceError(err) => Err(InvokeError::ServiceError(err))
                }
            },
            _ = timeout.tick() => {
                Err(InvokeError::Timeout)
            }
        }  
    }
}