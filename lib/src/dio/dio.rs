#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use tracing_futures::Instrument;
use error_chain::bail;
use crate::prelude::*;
use tokio::sync::broadcast;
use fxhash::FxHashMap;
use fxhash::FxHashSet;
use multimap::MultiMap;
use serde::{Deserialize};
use serde::{Serialize, Serializer, de::Deserializer, de::DeserializeOwned};
use std::{fmt::Debug, sync::Arc};
use parking_lot::Mutex as StdMutex;
use parking_lot::RwLock as StdRwLock;
use std::ops::Deref;
use std::ops::DerefMut;
use tokio::sync::mpsc;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Weak;

use crate::header::PrimaryKeyScope;
use super::DioMutState;
use super::row::*;
use super::dao::*;
use crate::meta::*;
use crate::event::*;
use crate::tree::*;
use crate::index::*;
use crate::transaction::*;
use crate::comms::*;
use crate::spec::*;
use crate::error::*;
use crate::trust::LoadResult;
use crate::lint::*;
use crate::time::*;

use crate::crypto::{EncryptedPrivateKey, PrivateSignKey};
use crate::{crypto::EncryptKey, session::{AteSession, AteSessionProperty}};

#[derive(Debug)]
pub(crate) struct DioState
{
    pub(super) cache_load: FxHashMap<PrimaryKey, (Arc<EventData>, EventLeaf)>,
}

/// Represents a series of mutations that the user is making on a particular chain-of-trust
/// with a specific set of facts attached to a session. All changes are stored in memory
/// until the commit function is invoked which will feed them into the chain.
///
/// If you decide to abort the transaction then call the `cancel` function before it goes
/// out of scope however if you mutate data and do not call `commit` then the data will be
/// lost (or an assert will be triggerd when in Debug mode).
///
/// These objects are multi-thread safe and allow for very high concurrency through async
/// operations.
///
/// When setting the scope for the DIO it will behave differently when the commit function
/// is invoked based on what scope you set for the transaction.
pub struct Dio
{
    pub(super) chain: Arc<Chain>,
    pub(super) multi: ChainMultiUser,
    pub(super) state: StdMutex<DioState>,
    pub(super) session: StdRwLock<AteSession>,
    pub(super) time: Arc<TimeKeeper>,
}

pub(crate) struct DioScope
{
    pop: Option<Arc<Dio>>,
    _negative: Rc<()>,
}

impl DioScope
{
    pub fn new(dio: &Arc<Dio>) -> Self {
        DioScope {
            pop: Dio::current_set(Some(Arc::clone(dio))),
            _negative: Rc::new(())
        }
    }
}

impl Drop
for DioScope
{
    fn drop(&mut self) {
        Dio::current_set(self.pop.take());
    }
}

pub(crate) enum DioWeak
{
    Uninitialized,
    Weak(Weak<Dio>)
}

impl Default
for DioWeak
{
    fn default() -> Self
    {
        match Dio::current_get() {
            Some(a) => DioWeak::Weak(Arc::downgrade(&a)),
            None => DioWeak::Uninitialized
        }
    }
}

impl Clone
for DioWeak
{
    fn clone(&self) -> Self
    {
        match self {
            Self::Uninitialized => Self::default(),
            Self::Weak(a) => Self::Weak(Weak::clone(a))
        }
    }
}

impl From<&Arc<Dio>>
for DioWeak
{
    fn from(val: &Arc<Dio>) -> Self
    {
        DioWeak::Weak(Arc::downgrade(val))
    }
}

impl From<&Arc<DioMut>>
for DioWeak
{
    fn from(val: &Arc<DioMut>) -> Self
    {
        DioWeak::Weak(Arc::downgrade(&val.dio))
    }
}

impl Dio
{
    thread_local! {
        static CURRENT: RefCell<Option<Arc<Dio>>> = RefCell::new(None)
    }

    pub(crate) fn current_get() -> Option<Arc<Dio>>
    {
        Dio::CURRENT.with(|dio| {
            let dio = dio.borrow();
            return dio.clone()
        })
    }

    fn current_set(val: Option<Arc<Dio>>) -> Option<Arc<Dio>>
    {
        Dio::CURRENT.with(|dio| {
            let mut dio = dio.borrow_mut();
            match val {
                Some(a) => dio.replace(a),
                None => dio.take()
            }
        })
    }

    pub fn chain(&self) -> &Arc<Chain> {
        &self.chain
    }

    async fn run_async<F>(&self, future: F) -> F::Output
    where F: std::future::Future,
    {
        let key_str = self.chain.key().to_string();
        TaskEngine::run_until(future
            .instrument(span!(Level::DEBUG, "dio", key=key_str.as_str()))
        ).await
    }

    pub async fn load_raw(self: &Arc<Self>, key: &PrimaryKey) -> Result<EventData, LoadError>
    {
        self.run_async(self.__load_raw(key)).await
    }

    pub(super) async fn __load_raw(self: &Arc<Self>, key: &PrimaryKey) -> Result<EventData, LoadError>
    {
        let leaf = match self.multi.lookup_primary(key).await {
            Some(a) => a,
            None => bail!(LoadErrorKind::NotFound(key.clone()))
        };
        let data = self.multi.load(leaf).await?.data;
        Ok(data)
    }

    pub async fn load<D>(self: &Arc<Self>, key: &PrimaryKey) -> Result<Dao<D>, LoadError>
    where D: DeserializeOwned,
    {
        self.run_async(self.__load(key)).await
    }

    pub(super) async fn __load<D>(self: &Arc<Self>, key: &PrimaryKey) -> Result<Dao<D>, LoadError>
    where D: DeserializeOwned,
    {
        {
            let state = self.state.lock();
            if let Some((dao, leaf)) = state.cache_load.get(key) {
                let (row_header, row) = Row::from_event(self, dao.deref(), leaf.created, leaf.updated)?;
                return Ok(Dao::new(self, row_header, row));
            }
        }

        let leaf = match self.multi.lookup_primary(key).await {
            Some(a) => a,
            None => bail!(LoadErrorKind::NotFound(key.clone()))
        };
        Ok(self.load_from_entry(leaf).await?)
    }

    pub async fn load_and_take<D>(self: &Arc<Self>, key: &PrimaryKey) -> Result<D, LoadError>
    where D: DeserializeOwned,
    {
        let ret: Dao<D> = self.load(key).await?;
        Ok(ret.take())
    }

    pub async fn exists(&self, key: &PrimaryKey) -> bool
    {
        self.run_async(self.__exists(key)).await
    }

    pub(super) async fn __exists(&self, key: &PrimaryKey) -> bool
    {
        {
            let state = self.state.lock();
            if let Some((_, _)) = state.cache_load.get(key) {
                return true;
            }
        }

        self.multi.lookup_primary(key).await.is_some()
    }

    pub(crate) async fn load_from_entry<D>(self: &Arc<Self>, leaf: EventLeaf)
    -> Result<Dao<D>, LoadError>
    where D: DeserializeOwned,
    {
        self.run_async(self.__load_from_entry(leaf)).await
    }

    pub(super) async fn __load_from_entry<D>(self: &Arc<Self>, leaf: EventLeaf)
    -> Result<Dao<D>, LoadError>
    where D: DeserializeOwned,
    {
        let evt = self.multi.load(leaf).await?;
        let session = self.session();

        Ok(self.load_from_event(session.as_ref(), evt.data, evt.header.as_header()?, leaf)?)
    }

    pub(crate) fn load_from_event<D>(self: &Arc<Self>, session: &AteSession, mut data: EventData, header: EventHeader, leaf: EventLeaf)
    -> Result<Dao<D>, LoadError>
    where D: DeserializeOwned,
    {
        data.data_bytes = match data.data_bytes {
            Some(data) => Some(self.multi.data_as_overlay(&header.meta, data, session)?),
            None => None,
        };

        let mut state = self.state.lock();
        match header.meta.get_data_key() {
            Some(key) =>
            {
                let (row_header, row) = Row::from_event(self, &data, leaf.created, leaf.updated)?;
                state.cache_load.insert(key.clone(), (Arc::new(data), leaf));
                Ok(Dao::new(self, row_header, row))
            },
            None => Err(LoadErrorKind::NoPrimaryKey.into())
        }
    }

    pub async fn children_keys(self: &Arc<Self>, parent_id: PrimaryKey, collection_id: u64) -> Result<Vec<PrimaryKey>, LoadError>
    {
        self.run_async(self.__children_keys(parent_id, collection_id)).await
    }

    pub async fn __children_keys(self: &Arc<Self>, parent_id: PrimaryKey, collection_id: u64) -> Result<Vec<PrimaryKey>, LoadError>
    {
        // Build the secondary index key
        let collection_key = MetaCollection {
            parent_id,
            collection_id,
        };

        // Build a list of keys
        let keys = match self.multi.lookup_secondary_raw(&collection_key).await {
            Some(a) => a,
            None => return Ok(Vec::new())
        };
        Ok(keys)
    }

    pub async fn children<D>(self: &Arc<Self>, parent_id: PrimaryKey, collection_id: u64) -> Result<Vec<Dao<D>>, LoadError>
    where D: DeserializeOwned,
    {
        self.children_ext(parent_id, collection_id, false, false).await
    }

    pub async fn children_ext<D>(self: &Arc<Self>, parent_id: PrimaryKey, collection_id: u64, allow_missing_keys: bool, allow_serialization_error: bool) -> Result<Vec<Dao<D>>, LoadError>
    where D: DeserializeOwned,
    {
        self.run_async(self.__children_ext(parent_id, collection_id, allow_missing_keys, allow_serialization_error)).await
    }

    pub(super) async fn __children_ext<D>(self: &Arc<Self>, parent_id: PrimaryKey, collection_id: u64, allow_missing_keys: bool, allow_serialization_error: bool) -> Result<Vec<Dao<D>>, LoadError>
    where D: DeserializeOwned,
    {
        // Load all the objects
        let keys = self.__children_keys(parent_id, collection_id).await?;
        Ok(self.__load_many_ext(keys.into_iter(), allow_missing_keys, allow_serialization_error).await?)
    }

    pub async fn load_many<D>(self: &Arc<Self>, keys: impl Iterator<Item=PrimaryKey>) -> Result<Vec<Dao<D>>, LoadError>
    where D: DeserializeOwned,
    {
        self.load_many_ext(keys, false, false).await
    }

    pub async fn load_many_ext<D>(self: &Arc<Self>, keys: impl Iterator<Item=PrimaryKey>, allow_missing_keys: bool, allow_serialization_error: bool) -> Result<Vec<Dao<D>>, LoadError>
    where D: DeserializeOwned,
    {
        self.run_async(self.__load_many_ext(keys, allow_missing_keys, allow_serialization_error)).await
    }

    pub(super) async fn __load_many_ext<D>(self: &Arc<Self>, keys: impl Iterator<Item=PrimaryKey>, allow_missing_keys: bool, allow_serialization_error: bool) -> Result<Vec<Dao<D>>, LoadError>
    where D: DeserializeOwned,
    {
        // This is the main return list
        let mut already = FxHashSet::default();
        let mut ret = Vec::new();

        let inside_async = self.multi.inside_async.read().await;
        
        // We either find existing objects in the cache or build a list of objects to load
        let to_load = {
            let mut to_load = Vec::new();

            let state = self.state.lock();
            for key in keys
            {
                if let Some((dao, leaf)) = state.cache_load.get(&key) {
                    let (row_header, row) = Row::from_event(self, dao.deref(), leaf.created, leaf.updated)?;
                    already.insert(row.key.clone());
                    ret.push(Dao::new(self, row_header, row));
                    continue;
                }

                to_load.push(match inside_async.chain.lookup_primary(&key) {
                    Some(a) => a,
                    None => { continue },
                });
            }
            to_load
        };

        // Load all the objects that have not yet been loaded
        let to_load = inside_async.chain.load_many(to_load).await?;

        // Now process all the objects
        let ret = {
            let mut state = self.state.lock();
            let session = self.session();
            for mut evt in to_load {

                let mut header = evt.header.as_header()?;

                let key = match header.meta.get_data_key() {
                    Some(k) => k,
                    None => { continue; }
                };

                if let Some((dao, leaf)) = state.cache_load.get(&key) {
                    let (row_header, row) = Row::from_event(self, dao.deref(), leaf.created, leaf.updated)?;

                    already.insert(row.key.clone());
                    ret.push(Dao::new(self, row_header, row));
                }
                
                let (row_header, row) = match self.__process_load_row(session.as_ref(), &mut evt, &mut header.meta, allow_missing_keys, allow_serialization_error)? {
                    Some(a) => a,
                    None => { continue; }
                };
                state.cache_load.insert(row.key.clone(), (Arc::new(evt.data), evt.leaf));

                already.insert(row.key.clone());
                ret.push(Dao::new(self, row_header, row));
            }
            ret
        };

        Ok(ret)
    }

    pub(crate) fn data_as_overlay(self: &Arc<Self>, session: &AteSession, data: &mut EventData) -> Result<(), TransformError>
    {
        data.data_bytes = match &data.data_bytes {
            Some(d) => Some(self.multi.data_as_overlay(&data.meta, d.clone(), session)?),
            None => None,
        };
        Ok(())
    }

    pub(super) fn __process_load_row<D>(self: &Arc<Self>, session: &AteSession, evt: &mut LoadResult, meta: &Metadata, allow_missing_keys: bool, allow_serialization_error: bool) -> Result<Option<(RowHeader, Row<D>)>, LoadError>
    where D: DeserializeOwned
    {
        evt.data.data_bytes = match &evt.data.data_bytes {
            Some(data) => {
                let data = match self.multi.data_as_overlay(meta, data.clone(), session) {
                    Ok(a) => a,
                    Err(TransformError(TransformErrorKind::MissingReadKey(hash), _)) if allow_missing_keys => {
                        trace!("Missing read key {} - ignoring row", hash);
                        return Ok(None);
                    }
                    Err(err) => {
                        bail!(LoadErrorKind::TransformationError(err.0));
                    }
                };
                Some(data)
            },
            None => { return Ok(None); },
        };

        let (row_header, row) = match Row::from_event(self, &evt.data, evt.leaf.created, evt.leaf.updated) {
            Ok(a) => a,
            Err(err) => {
                if allow_serialization_error {
                    debug!("Serialization error {} - ignoring row", err);
                    return Ok(None);
                }
                bail!(LoadErrorKind::SerializationError(err.0));
            }
        };
        Ok(Some((row_header, row)))
    }

    pub fn session<'a>(&'a self) -> DioSessionGuard<'a> {
        DioSessionGuard::new(self)
    }

    pub fn session_mut<'a>(&'a self) -> DioSessionGuardMut<'a> {
        DioSessionGuardMut::new(self)
    }

    pub async fn wait_for_accurate_timing(&self)
    {
        self.time.wait_for_high_accuracy().await;
    }

    pub(crate) fn run_decache(self: &Arc<Dio>, mut decache: broadcast::Receiver<Vec<PrimaryKey>>) {
        let dio = Arc::downgrade(self);

        TaskEngine::spawn(async move {
            loop {
                let recv = tokio::time::timeout(std::time::Duration::from_secs(1), decache.recv()).await;
                let dio = match Weak::upgrade(&dio) {
                    Some(a) => a,
                    None => { break; }
                };
                let recv = match recv {
                    Ok(a) => a,
                    Err(_) => { continue; }
                };
                let recv = match recv {
                    Ok(a) => a,
                    Err(_) => { break; }
                };

                let mut state = dio.state.lock();
                for key in recv {
                    state.cache_load.remove(&key);
                }
            }
        });
    }
}

pub struct DioSessionGuard<'a>
{
    lock: parking_lot::RwLockReadGuard<'a, AteSession>
}

impl<'a> DioSessionGuard<'a>
{
    fn new(dio: &'a Dio) -> DioSessionGuard<'a>
    {
        DioSessionGuard {
            lock: dio.session.read()
        }
    }

    pub fn as_ref(&self) -> &AteSession {
        self.lock.deref()
    }
}

impl<'a> Deref
for DioSessionGuard<'a>
{
    type Target = AteSession;

    fn deref(&self) -> &Self::Target {
        self.lock.deref()
    }
}

pub struct DioSessionGuardMut<'a>
{
    lock: parking_lot::RwLockWriteGuard<'a, AteSession>
}

impl<'a> DioSessionGuardMut<'a>
{
    fn new(dio: &'a Dio) -> DioSessionGuardMut<'a>
    {
        DioSessionGuardMut {
            lock: dio.session.write()
        }
    }

    pub fn as_ref(&self) -> &AteSession {
        self.lock.deref()
    }

    pub fn as_mut(&mut self) -> &mut AteSession {
        self.lock.deref_mut()
    }
}

impl<'a> Deref
for DioSessionGuardMut<'a>
{
    type Target = AteSession;

    fn deref(&self) -> &Self::Target {
        self.lock.deref()
    }
}

impl<'a> DerefMut
for DioSessionGuardMut<'a>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.lock.deref_mut()
    }
}

impl Chain
{
    /// Opens a data access layer that allows read only access to data within the chain
    /// In order to make changes to data you must use '.dio_mut', '.dio_fire', '.dio_full' or '.dio_trans'
    pub async fn dio(self: &Arc<Chain>, session: &'_ AteSession) -> Arc<Dio> {
        TaskEngine::run_until(self.__dio(session)).await
    }

    pub(crate) async fn __dio(self: &Arc<Chain>, session: &'_ AteSession) -> Arc<Dio> {
        let decache = self.decache.subscribe();
        let multi = self.multi().await;
        let ret = Dio {
            chain: Arc::clone(self),
            state: StdMutex::new(DioState {
                cache_load: FxHashMap::default(),
            }),
            session: StdRwLock::new(session.clone()),
            multi,
            time: Arc::clone(&self.time),
        };
        let ret = Arc::new(ret);
        ret.run_decache(decache);
        ret
    }
}

impl Dio
{
    pub async fn as_mut(self: &Arc<Self>) -> Arc<DioMut> {
        self.trans(TransactionScope::Local).await
    }

    pub async fn trans(self: &Arc<Self>, scope: TransactionScope) -> Arc<DioMut> {
        TaskEngine::run_until(self.__trans(scope)).await
    }

    pub(crate) async fn __trans(self: &Arc<Self>, scope: TransactionScope) -> Arc<DioMut> {
        DioMut::__new(self, scope).await
    }
}