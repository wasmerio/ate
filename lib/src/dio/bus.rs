use serde::{Serialize, de::DeserializeOwned};
use std::marker::PhantomData;
use tokio::sync::mpsc;

use crate::{error::*, event::*, meta::MetaCollection};
use super::dao::*;
use crate::dio::*;
use crate::chain::*;
use crate::index::*;
use crate::session::*;

impl<D> Dao<D>
where D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    #[allow(dead_code)]
    pub fn bus<'a, C>(&self, chain: &'a Chain, vec: DaoVec<C>) -> Bus<'a, C>
    where C: Serialize + DeserializeOwned + Clone + Send + Sync,
    {
        let vec = MetaCollection {
            parent_id: self.key().clone(),
            collection_id: vec.vec_id,
        };
        Bus::new(chain, vec)
    }
}

#[allow(dead_code)]
pub struct Bus<'a, D>
where D: Serialize + DeserializeOwned + Clone + Send + Sync
{
    id: u64,
    chain: &'a Chain,
    vec: MetaCollection,
    receiver: mpsc::Receiver<EventData>,
    _marker: PhantomData<D>,
}

impl<'a, D> Bus<'a, D>
where D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    pub(crate) fn new(chain: &'a Chain, vec: MetaCollection) -> Bus<'a, D>
    {
        let id = fastrand::u64(..);
        let (tx, rx) = mpsc::channel(100);
        
        {
            let mut lock = chain.inside_sync.write();
            let listener = ChainListener {
                id: id,
                sender: tx,
            };
            lock.listeners.insert(vec.clone(), listener);
        }

        Bus {
            id: fastrand::u64(..),
            chain: chain,
            vec: vec,
            receiver: rx,
            _marker: PhantomData,
        }
    }

    #[allow(dead_code)]
    pub async fn recv(&mut self, session: &AteSession) -> Result<D, BusError> {
        while let Some(mut evt) = self.receiver.recv().await {

            let multi = self.chain.multi().await;
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

    #[allow(dead_code)]
    pub async fn process(&mut self, dio: &mut Dio<'a>) -> Result<Dao<D>, BusError> {
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
            if dao.try_lock_then_delete(dio).await? == true {
                return Ok(dao);
            }
            dao.cancel();
        }
    }
}

impl<'a, D> Drop
for Bus<'a, D>
where D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    fn drop(&mut self)
    {
        let mut lock = self.chain.inside_sync.write();
        if let Some(vec) = lock.listeners.get_vec_mut(&self.vec) {
            if let Some(index) = vec.iter().position(|x| x.id == self.id) {
                vec.remove(index);
            }
        }
    }
}