#![allow(unused_imports)]
use async_trait::async_trait;
use log::{info, error, debug};
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

pub type ServiceInstance<REQ, RES> = Arc<dyn ServiceHandler<REQ, RES> + Send + Sync>;

pub struct InvocationContext<'a>
{
    pub dio: Dio<'a>,
    pub session: &'a Session,
    pub chain: Arc<Chain>,
}

#[async_trait]
pub trait ServiceHandler<REQ, RES>
where REQ: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized,
      RES: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized
{
    async fn process<'a>(&self, request: REQ, context: InvocationContext<'a>) -> RES;
}

pub(crate) struct ServiceHook<REQ, RES>
where REQ: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized,
      RES: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized
{
    chain: Weak<Chain>,
    session: Session,
    handler: ServiceInstance<REQ, RES>,
    request_type_name: String,
    response_type_name: String,
}

impl<REQ, RES> ServiceHook<REQ, RES>
where REQ: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized,
      RES: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized
{
    pub(crate) fn new(chain: &Arc<Chain>, session: Session, handler: ServiceInstance<REQ, RES>) -> ServiceHook<REQ, RES> {
        ServiceHook {
            chain: Arc::downgrade(chain),
            session: session.clone(),
            handler: Arc::clone(&handler),
            request_type_name: std::any::type_name::<REQ>().to_string(),
            response_type_name: std::any::type_name::<RES>().to_string(),
        }
    }
}

#[async_trait]
pub trait Service
where Self: Send + Sync
{
    fn filter(&self, evt: &EventData) -> bool;

    async fn notify(&self, key: PrimaryKey) -> Result<(), CommandError>;
}

#[async_trait]
impl<REQ, RES> Service
for ServiceHook<REQ, RES>
where REQ: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized,
      RES: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized
{
    fn filter(&self, evt: &EventData) -> bool {
        if let Some(t) = evt.meta.get_type_name() {
            return t.type_name == self.request_type_name;
        }
        false
    }

    async fn notify(&self, key: PrimaryKey) -> Result<(), CommandError>
    {
        // Get a reference to the chain
        let chain = match self.chain.upgrade() {
            Some(a) => a,
            None => {
                return Err(CommandError::Aborted);
            }
        };

        let res: RES =
        {
            // Load the object
            let mut dio = chain.dio(&self.session).await;
            let req = dio.load::<REQ>(&key).await?;

            // Create the context
            let context = InvocationContext
            {
                dio,
                session: &self.session,
                chain: Arc::clone(&chain),
            };

            // Invoke the callback in the service
            self.handler.process(req.take(), context).await
        };

        // Turn it into a data object to be stored on commit
        let mut dio = chain.dio(&self.session).await;
        let mut res = dio.store_ext(res, self.session.log_format.clone(), None, false)?;

        // Add the metadata
        res.add_extra_metadata(CoreMetadata::Type(MetaType {
            type_name: self.response_type_name.clone()
        }));
        res.add_extra_metadata(CoreMetadata::Reply(key));
        
        // Commit the transaction
        res.commit(&mut dio)?;
        dio.broadcast().await?;
        Ok(())
    }
}

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

    #[allow(dead_code)]
    pub fn add_service<REQ, RES>(self: &Arc<Self>, session: Session, handler: ServiceInstance<REQ, RES>)
    where REQ: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized + 'static,
          RES: Serialize + DeserializeOwned + Clone + Sync + Send + ?Sized + 'static
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
    pub(crate) async fn notify(self) -> Result<(), CommandError> {
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

pub(crate) async fn callback_events_notify(mut notifies: Vec<Notify>) -> Result<(), CommandError>
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
    impl super::ServiceHandler<Ping, Pong>
    for PingPongTable
    {
        async fn process<'a>(&self, ping: Ping, _context: InvocationContext<'a>) -> Pong
        {
            Pong { msg: ping.msg }
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
        let pong: Pong = chain.invoke(&session, Ping {
            msg: "hi".to_string()
        }).await?;

        debug!("received pong with msg [{}]", pong.msg);
        Ok(())
    }
}