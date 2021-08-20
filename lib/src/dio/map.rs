#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use tracing_futures::Instrument;
use error_chain::bail;
use std::marker::PhantomData;
use std::sync::{Arc, Weak};
use fxhash::FxHashMap;
use std::hash::Hash;
use std::fmt;
use std::collections::hash_map::Keys;
use std::collections::hash_map::Entry;
use std::collections::hash_map::OccupiedEntry;
use std::collections::hash_map::VacantEntry;

use serde::*;
use serde::de::*;
use super::dio::DioWeak;
use crate::dio::*;
use crate::dio::dao::*;
use crate::error::*;
use std::collections::VecDeque;
use crate::prelude::*;

/// Rerepresents a map of key and value attached to a parent DAO
///
/// This object does not actually store the values which are
/// actually stored within the chain-of-trust as seperate events
/// that are indexed by this map
///
#[derive(Serialize, Deserialize)]
pub struct DaoMap<K, V>
where K: Eq + Hash
{
    lookup: FxHashMap<K, DaoRef<V>>,
    pub(super) vec_id: u64,
    #[serde(skip)]
    pub(super) state: DaoMapState,
    #[serde(skip)]
    dio: DioWeak,
    #[serde(skip)]
    _phantom1: PhantomData<V>,
}

#[derive(Clone)]
pub(super) enum DaoMapState
{
    Unsaved,
    Saved(PrimaryKey)
}

impl Default
for DaoMapState
{
    fn default() -> Self
    {
        match PrimaryKey::current_get() {
            Some(a) => DaoMapState::Saved(a),
            None => DaoMapState::Unsaved
        }
    }
}

impl<K, V> std::fmt::Debug
for DaoMap<K, V>
where K: Eq + Hash,
      V: std::fmt::Debug
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let type_key_name = std::any::type_name::<K>();
        let type_value_name = std::any::type_name::<V>();
        write!(f, "dao-map(key_type={}, value_type={})", type_key_name, type_value_name)
    }
}

impl<K, V> Default
for DaoMap<K, V>
where K: Eq + Hash
{
    fn default() -> Self {
        DaoMap::new()
    }
}

impl<K, V> Clone
for DaoMap<K, V>
where K: Eq + Hash + Clone
{
    fn clone(&self) -> DaoMap<K, V>
    {
        DaoMap {
            vec_id: self.vec_id.clone(),
            lookup: self.lookup.clone(),
            state: self.state.clone(),
            dio: self.dio.clone(),
            _phantom1: PhantomData,
        }
    }
}

impl<K, V> DaoMap<K, V>
where K: Eq + Hash
{
    pub fn new() -> DaoMap<K, V> {
        DaoMap {
            vec_id: fastrand::u64(..),
            state: DaoMapState::Unsaved,
            dio: DioWeak::Uninitialized,
            lookup: FxHashMap::default(),
            _phantom1: PhantomData,
        }
    }
    
    pub fn dio(&self) -> Option<Arc<Dio>> {
        match &self.dio {
            DioWeak::Uninitialized => None,
            DioWeak::Weak(a) => Weak::upgrade(a)
        }
    }

    pub fn capacity(&self) -> usize {
        self.lookup.capacity()
    }

    pub fn keys(&self) -> KeysIter<'_, K> {
        let mut keys = self.lookup.keys().collect::<Vec<_>>();
        keys.reverse();
        KeysIter {
            keys,
        }
    }

    pub async fn iter(&self) -> Result<super::vec::Iter<V>, LoadError>
    where V: DeserializeOwned
    {
        self.iter_ext(false, false).await
    }

    pub async fn iter_ext(&self, allow_missing_keys: bool, allow_serialization_error: bool) -> Result<super::vec::Iter<V>, LoadError>
    where V: DeserializeOwned
    {
        let children = match &self.state {
            DaoMapState::Unsaved => vec![],
            DaoMapState::Saved(parent_id) =>
            {
                let dio = match self.dio() {
                    Some(a) => a,
                    None => bail!(LoadErrorKind::WeakDio)
                };
                
                dio.children_ext(parent_id.clone(), self.vec_id, allow_missing_keys, allow_serialization_error).await?
            },
        };

        Ok(
            super::vec::Iter::new(
            children
            )
        )
    }

    pub fn len(&self) -> usize {
        self.lookup.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() <= 0usize
    }

    pub async fn clear(&mut self, trans: &Arc<DioMut>) -> Result<(), LoadError> {
        for (_, v) in self.lookup.drain() {
            if v.is_some().await? {
                if let Some(id) = v.id {
                    trans.delete(&id).await?;
                }
            }
        }
        self.lookup.clear();
        Ok(())
    }

    pub fn reserve(&mut self, additional: usize) {
        self.lookup.reserve(additional);
    }

    pub async fn get(&self, k: &K) -> Result<Option<Dao<V>>, LoadError>
    where V: Serialize + DeserializeOwned
    {
        let val = self.lookup.get(k);
        Ok(
            match val {
                Some(a) => a.load().await?,
                None => None
            }
        )
    }

    pub async fn get_mut(&self, k: &K, trans: &Arc<DioMut>) -> Result<Option<DaoMut<V>>, LoadError>
    where V: Serialize + DeserializeOwned
    {
        let val = self.lookup.get(k);
        Ok(
            match val {
                Some(a) => match a.load().await? {
                    Some(b) => Some(b.as_mut(trans)),
                    None => None,
                },
                None => None
            }
        )
    }

    pub async fn get_key_value(&self, k: &K) -> Result<Option<(&K, Dao<V>)>, LoadError>
    where V: Serialize + DeserializeOwned
    {
        let val = self.lookup.get_key_value(k);
        Ok(
            match val {
                Some((k, v)) => match v.load().await? {
                    Some(b) => Some((k, b)),
                    None => None,
                },
                None => None
            }
        )
    }

    pub async fn contains_key(&self, k: &K) -> Result<bool, LoadError>
    {
        Ok(
            match self.lookup.get(k) {
                Some(a) => a.is_some().await?,
                None => false
            }
        )
    }

    pub async fn insert(&mut self, k: K, v: V, trans: &Arc<DioMut>) -> Result<(), AteError>
    where V: Serialize + DeserializeOwned
    {
        self.insert_ret(k, v, trans).await?;
        Ok(())
    }

    pub async fn insert_ret(&mut self, k: K, v: V, trans: &Arc<DioMut>) -> Result<DaoMut<V>, AteError>
    where V: Serialize + DeserializeOwned
    {
        let parent_id = match &self.state {
            DaoMapState::Unsaved => { bail!(AteErrorKind::SerializationError(SerializationErrorKind::SaveParentFirst)); },
            DaoMapState::Saved(a) => a.clone(),
        };

        let mut v = trans.store(v)?;
        v.attach_ext(parent_id, self.vec_id)?;

        let old = self.lookup.insert(k, DaoRef {
            id: Some(v.key().clone()),
            dio: DioWeak::Weak(Arc::downgrade(&trans.dio)),
            _marker: PhantomData,
        });
        if let Some(old) = old {
            if old.is_some().await? {
                if let Some(id) = old.id {
                    trans.delete(&id).await?;
                }
            }
        }
        Ok(v)
    }
    

    pub async fn remove(&mut self, k: &K, trans: &Arc<DioMut>) -> Result<(), LoadError>
    {
        if let Some(obj) = self.lookup.remove(k) {
            if obj.is_some().await? {
                if let Some(id) = obj.id {
                    trans.delete(&id).await?;
                }
            }
        }
        Ok(())
    }
}

pub struct KeysIter<'a, K>
{
    keys: Vec<&'a K>
}

impl<'a, K> Iterator
for KeysIter<'a, K>
{
    type Item = &'a K;

    fn next(&mut self) -> Option<Self::Item>
    {
        self.keys.pop()
    }
}