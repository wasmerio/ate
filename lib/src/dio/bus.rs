use error_chain::bail;
use serde::{de::DeserializeOwned, Serialize};
use std::marker::PhantomData;
use std::sync::Arc;
use std::ops::Deref;
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
use crate::header::PrimaryKey;
use crate::header::PrimaryKeyScope;
use crate::{error::*, event::*, meta::MetaCollection};

pub enum BusEvent<D>
{
    Updated(Dao<D>),
    Deleted(PrimaryKey),
}

impl<D> BusEvent<D>
{
    pub fn data(self) -> Option<D> {
        match self {
            BusEvent::Updated(data) => Some(data.take()),
            _ => None,
        }
    }
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
        }
    }
}

impl<D> PartialEq<BusEvent<D>>
for BusEvent<D>
where D: PartialEq<D>,
      Dao<D>: PartialEq<Dao<D>>
{
    fn eq(&self, other: &BusEvent<D>) -> bool {
        match self {
            BusEvent::Updated(dao1) => match other {
                BusEvent::Updated(dao2) => dao1.eq(dao2),
                _ => false
            },
            BusEvent::Deleted(key1) => match other {
                BusEvent::Deleted(key2) => key1.eq(key2),
                _ => false
            },
        }
    }
}

impl<D> Eq
for BusEvent<D>
where D: Eq + PartialEq<BusEvent<D>>,
      Dao<D>: PartialEq<Dao<D>>
{ }

pub enum TryBusEvent<D>
{
    Updated(Dao<D>),
    Deleted(PrimaryKey),
    NoData,
}

impl<D> TryBusEvent<D>
{
    pub fn data(self) -> Option<D> {
        match self {
            TryBusEvent::Updated(data) => Some(data.take()),
            _ => None,
        }
    }
}

impl<D> fmt::Debug
for TryBusEvent<D>
where D: fmt::Debug
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TryBusEvent::Updated(dao) => {
                write!(f, "updated(")?;
                dao.fmt(f)?;
                write!(f, ")")
            },
            TryBusEvent::Deleted(key) => {
                write!(f, "deleted({})", key)
            },
            TryBusEvent::NoData => {
                write!(f, "no-data")
            }

        }
    }
}

impl<D> PartialEq<TryBusEvent<D>>
for TryBusEvent<D>
where D: PartialEq<D>,
      Dao<D>: PartialEq<Dao<D>>
{
    fn eq(&self, other: &TryBusEvent<D>) -> bool {
        match self {
            TryBusEvent::Updated(dao1) => match other {
                TryBusEvent::Updated(dao2) => dao1.eq(dao2),
                _ => false
            },
            TryBusEvent::Deleted(key1) => match other {
                TryBusEvent::Deleted(key2) => key1.eq(key2),
                _ => false
            },
            TryBusEvent::NoData => match other {
                TryBusEvent::NoData => true,
                _ => false
            },
        }
    }
}

impl<D> Eq
for TryBusEvent<D>
where D: Eq + PartialEq<TryBusEvent<D>>,
      Dao<D>: PartialEq<Dao<D>>
{ }

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
        while let Some(evt) = self.receiver.recv().await {
            match self.ret_evt(evt).await? {
                TryBusEvent::Updated(dao) => {
                    return Ok(BusEvent::Updated(dao));
                },
                TryBusEvent::Deleted(key) => {
                    return Ok(BusEvent::Deleted(key));
                },
                _ => { continue; }
            }
        }
        Err(BusErrorKind::ChannelClosed.into())
    }

    pub async fn try_recv(&mut self) -> Result<TryBusEvent<D>, BusError>
    where
        D: DeserializeOwned,
    {
        loop {
            match self.receiver.try_recv() {
                Ok(evt) => {
                    match self.ret_evt(evt).await? {
                        TryBusEvent::Updated(dao) => {
                            return Ok(TryBusEvent::Updated(dao));
                        },
                        TryBusEvent::Deleted(key) => {
                            return Ok(TryBusEvent::Deleted(key));
                        },                        
                        TryBusEvent::NoData => {
                            return Ok(TryBusEvent::NoData);
                        }
                    }
                },
                Err(mpsc::error::TryRecvError::Empty) => {
                    return Ok(TryBusEvent::NoData);
                },
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    return Err(BusErrorKind::ChannelClosed.into());
                }
            }
        }
    }

    async fn ret_evt(&self, evt: EventWeakData) -> Result<TryBusEvent<D>, BusError>
    where
        D: DeserializeOwned,
    {
        if let Some(key) = evt.meta.get_tombstone() {
            return Ok(TryBusEvent::Deleted(key));
        }
        if evt.data_bytes.is_none() {
            return Ok(TryBusEvent::NoData);
        }

        let when = evt.meta.get_timestamp();
        let when = match when {
            Some(t) => t.time_since_epoch_ms,
            None => 0,
        };

        let data_key = evt
            .meta
            .get_data_key();

        let mut evt = EventStrongData {
            meta: evt.meta,
            data_bytes: match evt.data_bytes {
                MessageBytes::Some(a) => Some(a),
                MessageBytes::LazySome(l) => {
                    match self.chain.pipe.load_many(vec![l.record]).await {
                        Ok(data) => {
                            if let Some(data) =  data.into_iter().next() {
                                error!("BLAH BLAH!!!!");
                                data
                            } else {
                                return Ok(TryBusEvent::NoData);
                            }
                        }
                        Err(err) => {
                            trace!("bus recv failed to load - {}", err);
                            return Ok(TryBusEvent::NoData);
                        }
                    }
                },
                MessageBytes::None => None,
            },
            format: evt.format,
        };

        let session = self.dio.session();
        evt.data_bytes = match evt.data_bytes {
            Some(data) => Some(self.dio.multi.data_as_overlay(&evt.meta, data, session.deref())?),
            None => None,
        };

        let _pop1 = DioScope::new(&self.dio);
        let _pop2 = data_key
            .as_ref()
            .map(|a| PrimaryKeyScope::new(a.clone()));

        let (row_header, row) = super::row::Row::from_event(&self.dio, &evt, when, when)?;
        return Ok(TryBusEvent::Updated(Dao::new(&self.dio, row_header, row)));
    }
    
    pub async fn process(&mut self, trans: &Arc<DioMut>) -> Result<DaoMut<D>, BusError>
    where
        D: Serialize + DeserializeOwned,
    {
        loop {
            let dao = self.recv().await?;
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
