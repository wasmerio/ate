#![allow(unused_imports)]
use serde::{Serialize, de::DeserializeOwned};
use std::{marker::PhantomData, sync::Weak, time::Duration};
use tokio::sync::mpsc;
use tokio::select;
use std::sync::Arc;

use crate::{error::*, event::*, meta::{CoreMetadata, MetaCollection}};
use super::dao::*;
use crate::dio::*;
use crate::chain::*;
use crate::index::*;
use crate::session::*;
use crate::meta::*;
use crate::header::*;

impl Chain
{
    pub async fn invoke<C, R>(self: Arc<Self>, session: &Session, request: C) -> Result<R, CommandError>
    where C: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized,
          R: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized,
    {
        self.invoke_ext(session, request, std::time::Duration::from_secs(60)).await
    }

    pub async fn invoke_ext<C, R>(self: Arc<Self>, session: &Session, request: C, timeout: Duration) -> Result<R, CommandError>
    where C: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized,
          R: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized,
    {
        // Build the command object
        let mut dio = self.dio(session).await;
        let mut cmd = dio.store_ext(request, session.log_format, None, false)?;

        // Add the extra metadata about the type so the other side can find it
        cmd.add_extra_metadata(CoreMetadata::Type(MetaType {
            type_name: std::any::type_name::<C>().to_string()
        }));

        // Sniff out the response object
        let cmd_id = cmd.key().clone();
        let join = sniff_for_command(Arc::downgrade(&self), Box::new(move |h| {
            if let Some(reply) = h.meta.is_reply_to_what() {
                return reply == cmd_id;
            }
            false
        }));

        // Send our command
        cmd.commit(&mut dio)?;
        dio.commit().await?;
        
        // The caller will wait on the response from the sniff that is looking for a reply object
        let key = tokio::time::timeout(timeout, join).await?;
        let key = match key {
            Some(a) => a,
            None => { return Err(CommandError::Aborted); }
        };
        Ok(dio.load::<R>(&key).await?.take())
    }

    pub async fn service<C, R>(self: Arc<Self>, session: &Session, worker: impl Fn(&mut Dio, Dao<C>) -> R) -> Result<(), CommandError>
    where C: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized,
          R: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized,
    {
        // Downgrade our reference to a weak reference
        let weak = Arc::downgrade(&self);
        drop(self);

        // Enter a processing loop until the chain fails
        loop
        {
            // Sniff for the command object
            let type_name = std::any::type_name::<C>().to_string();
            let key = sniff_for_command(Weak::clone(&weak), Box::new(move |h| {
                if let Some(t) = h.meta.get_type_name() {
                    return t.type_name == type_name;
                }
                false
            })).await;

            // If its was aborted then we should give up
            let key = match key {
                Some(a) => a,
                None => { return Err(CommandError::Aborted); }
            };

            // Attempt to process this command on the chain
            if let Some(chain) = weak.upgrade()
            {
                // Load the command object
                let mut dio = chain.dio(session).await;
                let cmd = dio.load::<C>(&key).await?;

                // Process it in the worker
                let reply = worker(&mut dio, cmd);

                // Store the reply (with some extra metadata)
                let mut reply = dio.store_ext(reply, session.log_format, None, false)?;
                reply.add_extra_metadata(CoreMetadata::Type(MetaType {
                    type_name: std::any::type_name::<R>().to_string()
                }));
                reply.add_extra_metadata(CoreMetadata::Reply(key));
                
                // Send our reply then move onto the next message
                reply.commit(&mut dio)?;
                dio.commit().await?;
            }
            else
            {
                return Err(CommandError::Aborted);
            }
        }
    }
}

async fn sniff_for_command(chain: Weak<Chain>, what: Box<dyn Fn(&EventData) -> bool + Send + Sync>) -> Option<PrimaryKey>
{
    // Create a sniffer
    let (tx, mut rx) = mpsc::channel(1);
    let sniffer = ChainSniffer {
        filter: what,
        notify: tx,
    };

    // Insert a sniffer under a lock
    if let Some(chain) = chain.upgrade() {
        let mut guard = chain.inside_sync.write();
        guard.sniffers.push(sniffer);
    } else {
        return None;
    }

    // Now wait for the response
    rx.recv().await
}