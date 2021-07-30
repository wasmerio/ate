
#[allow(unused_imports)]
use log::{error, info, debug};
use serde::{Serialize, de::DeserializeOwned};
use std::marker::PhantomData;
use tokio::sync::mpsc;
use std::sync::Arc;

use crate::{error::*, event::*, meta::MetaCollection};
use super::dao::*;
use crate::dio::*;
use crate::chain::*;
use crate::index::*;
use crate::engine::*;
use crate::prelude::*;

impl<D> Dao<D>
where D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    #[allow(dead_code)]
    pub async fn bus<C>(&self, chain: &Arc<Chain>, vec: DaoVec<C>) -> Bus<C>
    where C: Serialize + DeserializeOwned + Clone + Send + Sync,
    {
        let vec = MetaCollection {
            parent_id: self.key().clone(),
            collection_id: vec.vec_id,
        };
        Bus::new(chain, vec).await
    }
}

#[allow(dead_code)]
pub struct Bus<D>
where D: Serialize + DeserializeOwned + Clone + Send + Sync
{
    chain: Arc<Chain>,
    vec: MetaCollection,
    receiver: mpsc::Receiver<EventData>,
    _marker: PhantomData<D>,
}

impl<D> Bus<D>
where D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    pub(crate) async fn new(chain: &Arc<Chain>, vec: MetaCollection) -> Bus<D>
    {
        let id = fastrand::u64(..);
        let (tx, rx) = mpsc::channel(100);
        
        {
            let mut lock = chain.inside_async.write().await;
            let listener = ChainListener {
                id: id,
                sender: tx,
            };
            lock.listeners.insert(vec.clone(), listener);
        }

        Bus {
            chain: Arc::clone(&chain),
            vec: vec,
            receiver: rx,
            _marker: PhantomData,
        }
    }

    pub async fn recv(&mut self, session: &'_ AteSession) -> Result<D, BusError> {
        TaskEngine::run_until(self.__recv(session)).await
    }

    async fn __recv(&mut self, session: &'_ AteSession) -> Result<D, BusError> {
        let multi = self.chain.multi().await;
        while let Some(mut evt) = self.receiver.recv().await {
            match evt.data_bytes {
                Some(data) => {
                    let data = multi.data_as_overlay(&mut evt.meta, data, session)?;
                    return Ok(evt.format.data.deserialize(&data)?)
                },
                None => continue,
            };
        }
        Err(BusError::ChannelClosed)
    }

    pub async fn process(&mut self, dio: &'_ mut Dio<'_>) -> Result<Dao<D>, BusError> {
        TaskEngine::run_until(self.__process(dio)).await
    }

    async fn __process(&mut self, dio: &'_ mut Dio<'_>) -> Result<Dao<D>, BusError> {
        loop {
            let mut dao: Dao<D> = match self.receiver.recv().await {
                Some(evt) => {
                    let header = evt.as_header()?;
                    let leaf = EventLeaf {
                        record: header.raw.event_hash,
                        created: 0,
                        updated: 0,
                    };
                    dio.load_from_event(evt, header, leaf)?
                },
                None => { return Err(BusError::ChannelClosed); }
            };
            dao.auto_cancel();
            if dao.try_lock_then_delete(dio).await? == true {
                return Ok(dao);
            }
        }
    }
}