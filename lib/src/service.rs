#![allow(unused_imports)]
use log::{info, error, warn, debug};
use async_trait::async_trait;
use serde::{Serialize, de::DeserializeOwned};
use std::{marker::PhantomData, sync::Weak, time::Duration};
use tokio::sync::mpsc;
use tokio::select;
use std::sync::Arc;
use parking_lot::RwLock as StdRwLock;
use parking_lot::RwLockWriteGuard as StdRwLockWriteGuard;
use parking_lot::RwLockReadGuard as StdRwLockReadGuard;

use crate::{error::*, event::*, mesh::MeshSession, meta::{CoreMetadata, MetaCollection}};
use crate::dio::*;
use crate::chain::*;
use crate::index::*;
use crate::session::*;
use crate::meta::*;
use crate::header::*;
use crate::repository::*;

pub type ServiceInstance<REQ, RES, ERR> = Arc<dyn ServiceHandler<REQ, RES, ERR> + Send + Sync>;

pub struct InvocationContext<'a>
{
    pub session: &'a Session,
    pub chain: Arc<Chain>,
    pub repository: Arc<dyn ChainRepository>
}

#[async_trait]
pub trait ServiceHandler<REQ, RES, ERR>
where REQ: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized,
      RES: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized,
      ERR: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized
{
    async fn process<'a>(&self, request: REQ, context: InvocationContext<'a>) -> Result<RES, ServiceError<ERR>>;
}

pub(crate) struct ServiceHook<REQ, RES, ERR>
where REQ: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized,
      RES: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized,
      ERR: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized
{
    chain: Weak<Chain>,
    session: Session,
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
    pub(crate) fn new(chain: &Arc<Chain>, session: Session, handler: ServiceInstance<REQ, RES, ERR>) -> ServiceHook<REQ, RES, ERR> {
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
pub trait Service
where Self: Send + Sync
{
    fn filter(&self, evt: &EventData) -> bool;

    async fn notify(&self, key: PrimaryKey) -> Result<(), ServiceError<()>>;
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
                warn!("service call failed - repository pointer is missing");
                return Ok(());
            }
        };

        let ret = {
            // Load the object
            let mut dio = chain.dio(&self.session).await;
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
                chain: Arc::clone(&chain),
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
        let mut res = dio.store_ext(res, self.session.log_format.clone(), None, false)?;

        // If the session has an encryption key then use it
        if let Some(key) = self.session.read_keys().into_iter().map(|a| a.clone()).next() {
            res.auth_mut().read = ReadOption::Specific(key.hash());
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

impl Chain
{
    pub async fn invoke<REQ, RES, ERR>(self: Arc<Self>, request: REQ) -> Result<RES, InvokeError<ERR>>
    where REQ: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized,
          RES: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized,
          ERR: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized,
    {
        self.invoke_ext(None, request, std::time::Duration::from_secs(30)).await
    }

    pub async fn invoke_ext<REQ, RES, ERR>(self: Arc<Self>, session: Option<&Session>, request: REQ, timeout: Duration) -> Result<RES, InvokeError<ERR>>
    where REQ: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized,
          RES: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized,
          ERR: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized,
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
        let mut dio = self.dio(session).await;
        let mut cmd = dio.store_ext(request, session.log_format, None, false)?;
        
        // Add an encryption key on the command (if the session has one)
        if let Some(key) = session.read_keys().into_iter().next() {
            cmd.auth_mut().read = ReadOption::Specific(key.hash());
        }

        // Add the extra metadata about the type so the other side can find it
        cmd.add_extra_metadata(CoreMetadata::Type(MetaType {
            type_name: std::any::type_name::<REQ>().to_string()
        }));

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
        cmd.commit(&mut dio)?;
        dio.commit().await?;
        
        // The caller will wait on the response from the sniff that is looking for a reply object
        let mut timeout = tokio::time::interval(timeout);
        timeout.tick().await;
        select! {
            key = join_res => {
                let key = match key {
                    Some(a) => a,
                    None => { return Err(InvokeError::Aborted); }
                };
                Ok(dio.load::<RES>(&key).await?.take())
            },
            key = join_err => {
                let key = match key {
                    Some(a) => a,
                    None => { return Err(InvokeError::Aborted); }
                };
                match dio.load::<ServiceErrorReply<ERR>>(&key).await?.take() {
                    ServiceErrorReply::Reply(e) => Err(InvokeError::Reply(e)),
                    ServiceErrorReply::ServiceError(err) => Err(InvokeError::ServiceError(err))
                }
            },
            _ = timeout.tick() => {
                Err(InvokeError::Timeout)
            }
        }  
    }

    #[allow(dead_code)]
    pub fn add_service<REQ, RES, ERR>(self: &Arc<Self>, session: Session, handler: ServiceInstance<REQ, RES, ERR>)
    where REQ: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized + 'static,
          RES: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized + 'static,
          ERR: std::fmt::Debug + Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized + 'static,
    {
        let mut guard = self.inside_sync.write();
        guard.services.push(
            Arc::new(ServiceHook::new(
                self,
                session,
                Arc::clone(&handler),
            ))
        );
    }
}

pub(crate) struct ChainSniffer
{
    pub(crate) id: u64,
    pub(crate) filter: Box<dyn Fn(&EventData) -> bool + Send + Sync>,
    pub(crate) notify: mpsc::Sender<PrimaryKey>,
}

impl ChainSniffer
{
    fn convert(&self, key: PrimaryKey) -> Notify {
        Notify {
            key,
            who: NotifyWho::Sender(self.notify.clone())
        }
    }
}

pub(crate) enum NotifyWho
{
    Sender(mpsc::Sender<PrimaryKey>),
    Service(Arc<dyn Service>)
}

pub(crate) struct Notify
{
    pub(crate) key: PrimaryKey,
    pub(crate) who: NotifyWho,
}

impl Notify
{
    pub(crate) async fn notify(self) -> Result<(), ServiceError<()>> {
        match self.who {
            NotifyWho::Sender(sender) => sender.send(self.key).await?,
            NotifyWho::Service(service) => service.notify(self.key).await?
        }
        Ok(())
    }
}

pub(crate) fn callback_events_prepare(guard: &StdRwLockReadGuard<ChainProtectedSync>, events: &Vec<EventData>) -> Vec<Notify>
{
    let mut ret = Vec::new();
    
    for sniffer in guard.sniffers.iter() {
        if let Some(key) = events.iter().filter_map(|e| match (*sniffer.filter)(e) {
            true => e.meta.get_data_key(),
            false => None,
        }).next() {
            ret.push(sniffer.convert(key));
        }
    }

    for service in guard.services.iter() {
        for key in events.iter().filter(|e| service.filter(&e)).filter_map(|e| e.meta.get_data_key()) {
            ret.push(Notify {
                key,
                who: NotifyWho::Service(Arc::clone(service))
            });
        }
    }

    ret
}

pub(crate) async fn callback_events_notify(mut notifies: Vec<Notify>) -> Result<(), ServiceError<()>>
{
    for notify in notifies.drain(..) {
        tokio::spawn(notify.notify());
    }
    Ok(())
}

async fn sniff_for_command(chain: Weak<Chain>, what: Box<dyn Fn(&EventData) -> bool + Send + Sync>) -> Option<PrimaryKey>
{
    // Create a sniffer
    let id = fastrand::u64(..);
    let (tx, mut rx) = mpsc::channel(1);
    let sniffer = ChainSniffer {
        id,
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
    let ret = rx.recv().await;

    // Remove the sniffer
    if let Some(chain) = chain.upgrade() {
        let mut guard = chain.inside_sync.write();
        guard.sniffers.retain(|s| s.id != id);
    }

    // Return the result
    ret
}

#[cfg(test)]
mod tests 
{
    #![allow(unused_imports)]
    use async_trait::async_trait;
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
    use crate::service::*;

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

    #[derive(Default)]
    struct PingPongTable
    {        
    }

    #[async_trait]
    impl super::ServiceHandler<Ping, Pong, Noise>
    for PingPongTable
    {
        async fn process<'a>(&self, ping: Ping, _context: InvocationContext<'a>) -> Result<Pong, ServiceError<Noise>>
        {
            Ok(Pong { msg: ping.msg })
        }
    }

    #[tokio::main]
    #[test]
    async fn test_service() -> Result<(), AteError>
    {
        crate::utils::bootstrap_env();

        debug!("creating test chain");
        let mut mock_cfg = crate::conf::mock_test_config();
        let chain = Arc::new(crate::trust::create_test_chain(&mut mock_cfg, "test_chain".to_string(), true, true, None).await);
        
        debug!("start the service on the chain");
        let session = Session::new(&mock_cfg);
        chain.add_service(session.clone(), Arc::new(PingPongTable::default()));
        
        debug!("sending ping");
        let pong: Result<Pong, InvokeError<Noise>> = chain.invoke(Ping {
            msg: "hi".to_string()
        }).await;

        debug!("received pong with msg [{}]", pong?.msg);
        Ok(())
    }
}