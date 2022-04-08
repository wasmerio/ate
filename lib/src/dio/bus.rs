use error_chain::bail;
use serde::{de::DeserializeOwned, Serialize};
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::mpsc;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use std::fmt;

use super::dao::*;
use super::dao_mut::*;
use super::dio_mut::*;
use super::vec::DaoVecState;
use super::*;
use crate::chain::*;
use crate::engine::*;
use crate::header::PrimaryKey;
use crate::header::PrimaryKeyScope;
use crate::{error::*, event::*, meta::MetaCollection};

pub enum BusEvent<D>
{
    Updated(Dao<D>),
    Deleted(PrimaryKey),
    LoadError(PrimaryKey, LoadError),
    NoData,
}

impl<D> fmt::Debug
for BusEvent<D>
where D: fmt::Debug
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BusEvent::Updated(dao) => {
                write!(f, "updated(")?;
                dao.fmt(f)?;
                write!(f, ")")
            },
            BusEvent::Deleted(key) => {
                write!(f, "deleted({})", key)
            },
            BusEvent::LoadError(key, err) => {
                write!(f, "load-error(key={},err={})", key, err)
            },
            BusEvent::NoData => {
                write!(f, "no-data")
            }

        }
    }
}

#[allow(dead_code)]
pub struct Bus<D> {
    dio: Arc<Dio>,
    chain: Arc<Chain>,
    vec: MetaCollection,
    receiver: mpsc::Receiver<EventWeakData>,
    _marker: PhantomData<D>,
}

impl<D> Bus<D> {
    pub(crate) async fn new(dio: &Arc<Dio>, vec: MetaCollection) -> Bus<D> {
        let id = fastrand::u64(..);
        let (tx, rx) = mpsc::channel(100);

        {
            let mut lock = dio.chain().inside_async.write().await;
            let listener = ChainListener { id: id, sender: tx };
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

    pub async fn recv(&mut self) -> Result<BusEvent<D>, BusError>
    where
        D: DeserializeOwned,
    {
        TaskEngine::run_until(self.__recv()).await
    }

    async fn __recv(&mut self) -> Result<BusEvent<D>, BusError>
    where
        D: DeserializeOwned,
    {
        while let Some(evt) = self.receiver.recv().await {
            match self.ret_evt(evt).await? {
                BusEvent::Updated(dao) => {
                    return Ok(BusEvent::Updated(dao));
                },
                BusEvent::Deleted(key) => {
                    return Ok(BusEvent::Deleted(key));
                },
                BusEvent::LoadError(key, err) => {
                    return Ok(BusEvent::LoadError(key, err));
                },
                _ => { continue; }
            }
        }
        Err(BusErrorKind::ChannelClosed.into())
    }

    pub async fn try_recv(&mut self) -> Result<BusEvent<D>, BusError>
    where
        D: DeserializeOwned,
    {
        TaskEngine::run_until(self.__try_recv()).await
    }

    pub async fn __try_recv(&mut self) -> Result<BusEvent<D>, BusError>
    where
        D: DeserializeOwned,
    {
        loop {
            match self.receiver.try_recv() {
                Ok(evt) => {
                    match self.ret_evt(evt).await? {
                        BusEvent::Updated(dao) => {
                            return Ok(BusEvent::Updated(dao));
                        },
                        BusEvent::Deleted(key) => {
                            return Ok(BusEvent::Deleted(key));
                        },
                        BusEvent::LoadError(key, err) => {
                            return Ok(BusEvent::LoadError(key, err));
                        },
                        BusEvent::NoData => {
                            return Ok(BusEvent::NoData);
                        }
                        _ => { continue; }
                    }
                },
                Err(mpsc::error::TryRecvError::Empty) => {
                    return Ok(BusEvent::NoData);
                },
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    return Err(BusErrorKind::ChannelClosed.into());
                }
            }
        }
    }

    async fn ret_evt(&self, evt: EventWeakData) -> Result<BusEvent<D>, BusError>
    where
        D: DeserializeOwned,
    {
        if let Some(key) = evt.meta.get_tombstone() {
            return Ok(BusEvent::Deleted(key));
        }
        if evt.data_bytes.is_none() {
            return Ok(BusEvent::NoData);
        }

        let when = evt.meta.get_timestamp();
        let when = match when {
            Some(t) => t.time_since_epoch_ms,
            None => 0,
        };

        let _pop1 = DioScope::new(&self.dio);
        let _pop2 = evt
            .meta
            .get_data_key()
            .as_ref()
            .map(|a| PrimaryKeyScope::new(a.clone()));

        let evt = EventStrongData {
            meta: evt.meta,
            data_bytes: match evt.data_bytes {
                MessageBytes::Some(a) => Some(a),
                MessageBytes::LazySome(l) => {
                    match self.chain.pipe.load_many(vec![l.record]).await {
                        Ok(data) => {
                            if let Some(data) =  data.into_iter().next() {
                                data
                            } else {
                                return Ok(BusEvent::NoData);
                            }
                        }
                        Err(err) => {
                            trace!("bus recv failed to load - {}", err);
                            return Ok(BusEvent::NoData);
                        }
                    }
                },
                MessageBytes::None => None,
            },
            format: evt.format,
        };

        let (row_header, row) = super::row::Row::from_event(&self.dio, &evt, when, when)?;
        return Ok(BusEvent::Updated(Dao::new(&self.dio, row_header, row)));
    }

    pub async fn process(&mut self, trans: &Arc<DioMut>) -> Result<DaoMut<D>, BusError>
    where
        D: Serialize + DeserializeOwned,
    {
        let trans = Arc::clone(&trans);
        TaskEngine::run_until(self.__process(&trans)).await
    }

    async fn __process(&mut self, trans: &Arc<DioMut>) -> Result<DaoMut<D>, BusError>
    where
        D: Serialize + DeserializeOwned,
    {
        loop {
            let dao = self.__recv().await?;
            if let BusEvent::Updated(dao) = dao {
                let mut dao = DaoMut::new(Arc::clone(trans), dao);
                if dao.try_lock_then_delete().await? == true {
                    return Ok(dao);
                }
            }
        }
    }
}

impl<D> DaoVec<D> {
    pub async fn bus(&self) -> Result<Bus<D>, BusError> {
        let parent_id = match &self.state {
            DaoVecState::Unsaved => {
                bail!(BusErrorKind::SaveParentFirst);
            }
            DaoVecState::Saved(a) => a.clone(),
        };

        let vec = MetaCollection {
            parent_id: parent_id,
            collection_id: self.vec_id,
        };

        let dio = match self.dio() {
            Some(a) => a,
            None => {
                bail!(BusErrorKind::WeakDio);
            }
        };

        Ok(Bus::new(&dio, vec).await)
    }
}
