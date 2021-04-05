#![allow(unused_imports)]
use log::{info, error, debug};
use serde::{Serialize, de::DeserializeOwned};
use std::{marker::PhantomData, sync::Weak, time::Duration};
use tokio::sync::mpsc;
use tokio::select;
use std::sync::Arc;

use crate::{error::*, event::*, mesh::MeshSession, meta::{CoreMetadata, MetaCollection}};
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
        let response_type_name = std::any::type_name::<R>().to_string();
        let join = sniff_for_command(Arc::downgrade(&self), Box::new(move |h| {
            if let Some(reply) = h.meta.is_reply_to_what() {
                if reply == cmd_id {
                    if let Some(t) = h.meta.get_type_name() {
                        return t.type_name == response_type_name;
                    }
                }
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

    pub async fn service<C, R>(self: &Arc<Self>, session: Session, worker: Box<dyn Fn(&mut Dio, C) -> R + Send + Sync>) -> Result<(), CommandError>
    where C: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized + 'static,
          R: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized + 'static,
    {
        // First we have to add the hook
        let (tx, mut rx) = mpsc::channel(100);
        {
            // The sniffer has a built in function the filters to only the events we care about
            let request_type_name = std::any::type_name::<C>().to_string();
            let sniffer = ChainSniffer {
                filter: Box::new(move |h| {
                    if let Some(t) = h.meta.get_type_name() {
                        return t.type_name == request_type_name;
                    }
                    false
                }),
                notify: tx,
            };

            // Insert a sniffer under a lock
            let mut guard = self.inside_sync.write();
            guard.eternal_sniffers.push(sniffer);
        }

        // Downgrade our reference to a weak reference
        let weak = Arc::downgrade(&self);

        // Enter a processing loop until the chain fails
        tokio::spawn( async move {
            loop
            {
                // Wait for a command to come in
                let key = rx.recv().await;

                // If its was aborted then we should give up
                let key = match key {
                    Some(a) => a,
                    None => {
                        debug!("service exited - channel closed");
                        return;
                    }
                };

                // Attempt to process this command on the chain
                if let Some(chain) = weak.upgrade()
                {
                    // Load the command object
                    let mut dio = chain.dio(&session).await;
                    if let Some(mut cmd) = eat_load(dio.load::<C>(&key).await)
                    {
                        // Attempt to lock the object (if that fails then someone else is processing it
                        match eat_lock(cmd.try_lock_then_delete(&mut dio).await) {
                            None => continue,
                            Some(false) => continue,
                            _ => { }
                        };

                        // Process it in the worker (deleting the row just before we do)
                        eat_serialization(cmd.commit(&mut dio));
                        let cmd = cmd.take();
                        let reply = worker(&mut dio, cmd);

                        // Store the reply (with some extra metadata)
                        if let Some(mut reply) = eat_serialization(dio.store_ext(reply, session.log_format, None, false))
                        {
                            reply.add_extra_metadata(CoreMetadata::Type(MetaType {
                                type_name: std::any::type_name::<R>().to_string()
                            }));
                            reply.add_extra_metadata(CoreMetadata::Reply(key));
                            
                            // Send our reply then move onto the next message
                            eat_serialization(reply.commit(&mut dio));
                            eat_commit(dio.commit().await);
                        }
                    }
                }
                else
                {
                    debug!("service exited - chain destroyed");
                    return;
                }
            }
        });

        // Everything is running
        Ok(())
    }
}

impl MeshSession
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
        let chain = Arc::clone(&self.chain);
        chain.invoke_ext(session, request, timeout).await
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

#[cfg(test)]
mod tests 
{
    #![allow(unused_imports)]
    use log::{info, error, debug};
    use serde::{Serialize, Deserialize};
    use std::sync::Arc;
    use crate::dio::*;
    use crate::chain::*;
    use crate::index::*;
    use crate::session::*;
    use crate::meta::*;
    use crate::header::*;
    use crate::error::*;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct Ping
    {
        msg: String
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct Pong
    {
        msg: String
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct Noise
    {
        dummy: u64
    }

    #[tokio::main]
    #[test]
    async fn test_cmd() -> Result<(), AteError>
    {
        crate::utils::bootstrap_env();

        debug!("creating test chain");
        let mut mock_cfg = crate::conf::mock_test_config();
        let chain = Arc::new(crate::trust::create_test_chain(&mut mock_cfg, "test_chain".to_string(), true, true, None).await);
        
        debug!("start the service on the chain");
        let session = Session::new(&mock_cfg);
        chain.service(session.clone(), Box::new(
            |_dio, p: Ping| Pong { msg: p.msg }
        )).await?;
        
        debug!("sending ping");
        let pong: Pong = chain.invoke(&session, Ping {
            msg: "hi".to_string()
        }).await?;

        debug!("received pong with msg [{}]", pong.msg);
        Ok(())
    }
}