
#[allow(unused_imports)]
use log::{error, info, debug};
use serde::{Serialize, de::DeserializeOwned};
use std::marker::PhantomData;
use tokio::sync::mpsc;
use std::sync::Arc;

use super::*;
use super::dio_mut::*;
use crate::header::PrimaryKeyScope;
use crate::{error::*, event::*, meta::MetaCollection};
use super::dao_mut::*;
use super::dao::*;
use crate::chain::*;
use crate::engine::*;
use super::vec::DaoVecState;

#[allow(dead_code)]
pub struct Bus<D>
{
    dio: Arc<Dio>,
    chain: Arc<Chain>,
    vec: MetaCollection,
    receiver: mpsc::Receiver<EventData>,
    _marker: PhantomData<D>,
}

impl<D> Bus<D>
{
    pub(crate) async fn new(dio: &Arc<Dio>, vec: MetaCollection) -> Bus<D>
    {
        let id = fastrand::u64(..);
        let (tx, rx) = mpsc::channel(100);
        
        {
            let mut lock = dio.chain().inside_async.write().await;
            let listener = ChainListener {
                id: id,
                sender: tx,
            };
            lock.listeners.insert(vec.clone(), listener);
        }

        Bus {
            dio: Arc::clone(&dio),
            chain: Arc::clone(dio.chain()),
            vec: vec,
            receiver: rx,
            _marker: PhantomData,
        }
    }

    pub async fn recv(&mut self) -> Result<Dao<D>, BusError>
    where D: DeserializeOwned
    {
        TaskEngine::run_until(self.__recv()).await
    }

    async fn __recv(&mut self) -> Result<Dao<D>, BusError>
    where D: DeserializeOwned
    {
        while let Some(evt) = self.receiver.recv().await {
            if evt.data_bytes.is_some() {
                continue;
            }

            let when = evt.meta.get_timestamp();
            let when = match when {
                Some(t) => t.time_since_epoch_ms,
                None => 0
            };

            let _pop1 = DioScope::new(&self.dio);
            let _pop2 = evt.meta.get_data_key().as_ref().map(|a| PrimaryKeyScope::new(a.clone()));

            let (row_header, row) = super::row::Row::from_event(&self.dio, &evt, when, when)?;
            return Ok(Dao::new(&self.dio, row_header, row));
        }
        Err(BusError::ChannelClosed)
    }

    pub async fn process(&mut self, trans: &Arc<DioMut>) -> Result<DaoMut<D>, BusError>
    where D: Serialize + DeserializeOwned
    {
        let trans = Arc::clone(&trans);
        TaskEngine::run_until(self.__process(&trans)).await
    }

    async fn __process(&mut self, trans: &Arc<DioMut>) -> Result<DaoMut<D>, BusError>
    where D: Serialize + DeserializeOwned
    {
        loop {
            let dao = self.__recv().await?;
            let mut dao = DaoMut::new(Arc::clone(trans), dao);
            if dao.try_lock_then_delete().await? == true {
                return Ok(dao);
            }
        }
    }
}

impl<D> DaoVec<D>
{
    pub async fn bus(&self) -> Result<Bus<D>, BusError>
    {
        let parent_id = match &self.state {
            DaoVecState::Unsaved => { return Err(BusError::SaveParentFirst); },
            DaoVecState::Saved(a) => a.clone(),
        };

        let vec = MetaCollection {
            parent_id: parent_id,
            collection_id: self.vec_id,
        };
        
        let dio = match self.dio() {
            Some(a) => a,
            None => { return Err(BusError::WeakDio); }
        };

        Ok(Bus::new(&dio, vec).await)
    }
}