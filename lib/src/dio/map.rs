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

    pub async fn values(&self)
    -> Result<ValuesIter<V>, LoadError>
    where V: Serialize + DeserializeOwned
    {
        let mut ret = Vec::new();
        for value in self.lookup.values() {
            if let Some(v) = value.load().await? {
                ret.push(v);
            }
        }
        ret.reverse();
        Ok(ValuesIter {
            values: ret
        })
    }

    pub async fn values_mut(&self, trans: &Arc<DioMut>)
    -> Result<ValuesMutIter<V>, LoadError>
    where V: Serialize + DeserializeOwned
    {
        let mut ret = Vec::new();
        for value in self.lookup.values() {
            if let Some(v) = value.load().await? {
                ret.push(v.as_mut(trans));
            }
        }
        ret.reverse();
        Ok(ValuesMutIter {
            values: ret
        })
    }

    pub async fn iter(&self)
    -> Result<Iter<'_, K, V>, LoadError>
    where V: Serialize + DeserializeOwned
    {
        let mut ret = Vec::new();
        for (k, v) in self.lookup.iter() {
            if let Some(v) = v.load().await? {
                ret.push((k, v));
            }
        }
        ret.reverse();
        Ok(Iter {
            values: ret
        })
    }

    pub async fn drain(&mut self) -> Result<Drain<K, V>, LoadError>
    where V: Serialize + DeserializeOwned
    {
        let mut ret = Vec::new();
        for (k, v) in self.lookup.drain() {
            if let Some(v) = v.load().await? {
                ret.push((k, v));
            }
        }
        ret.reverse();
        Ok(Drain {
            values: ret
        })
    }

    pub async fn iter_mut(&self, trans: &Arc<DioMut>)
    -> Result<IterMut<'_, K, V>, LoadError>
    where V: Serialize + DeserializeOwned
    {
        let mut ret = Vec::new();
        for (k, v) in self.lookup.iter() {
            if let Some(v) = v.load().await? {
                ret.push((k, v.as_mut(trans)));
            }
        }
        ret.reverse();
        Ok(IterMut {
            values: ret
        })
    }

    pub fn len(&self) -> usize {
        self.lookup.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lookup.is_empty()
    }

    pub fn clear(&mut self) {
        self.lookup.clear();
    }

    pub fn reserve(&mut self, additional: usize) {
        self.lookup.reserve(additional);
    }

    pub fn shrink_to_fit(&mut self) {
        self.lookup.shrink_to_fit();
    }

    pub fn entry(&mut self, key: K) -> Entry<'_, K, DaoRef<V>> {
        self.lookup.entry(key)
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

    pub async fn get_key_value(&self, k: &K) -> Result<Option<(&K, Dao<V>)>, LoadError>
    where V: Serialize + DeserializeOwned
    {
        let val = self.lookup.get_key_value(k);
        Ok(
            match val {
                Some((k, v)) => match v.load().await? {
                    Some(b) => Some((k, b)),
                    None => None
                },
                None => None
            }
        )
    }

    pub fn contains_key(&self, k: &K) -> bool
    {
        self.lookup.contains_key(k)
    }

    pub async fn get_mut(&self, k: &K, trans: &Arc<DioMut>) -> Result<Option<DaoMut<V>>, LoadError>
    where V: Serialize + DeserializeOwned
    {
        let val = self.lookup.get(k);
        Ok(
            match val {
                Some(a) => match a.load().await? {
                    Some(a) => Some(a.as_mut(trans)),
                    None => None
                },
                None => None
            }
        )
    }

    pub fn insert(&mut self, k: K, v: V, trans: &Arc<DioMut>) -> Result<Option<DaoRef<V>>, SerializationError>
    where V: Serialize + DeserializeOwned
    {
        let v = trans.store(v)?;
        let old = self.lookup.insert(k, DaoRef {
            id: Some(v.key().clone()),
            dio: DioWeak::Weak(Arc::downgrade(&trans.dio)),
            _marker: PhantomData,
        });
        Ok(old)
    }

    pub fn remove(&mut self, k: &K) -> Option<DaoRef<V>>
    {
        self.lookup.remove(k)
    }

    pub fn remove_entry(&mut self, k: &K) -> Option<(K, DaoRef<V>)>
    {
        self.lookup.remove_entry(k)
    }

    pub fn retain<F>(&mut self, f: F)
    where F: FnMut(&K, &mut DaoRef<V>) -> bool,
    {
        self.lookup.retain(f)
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

pub struct ValuesIter<V>
{
    values: Vec<Dao<V>>
}

impl<V> Iterator
for ValuesIter<V>
{
    type Item = Dao<V>;

    fn next(&mut self) -> Option<Self::Item>
    {
        self.values.pop()
    }
}

pub struct ValuesMutIter<V>
where V: Serialize
{
    values: Vec<DaoMut<V>>
}

impl<V> Iterator
for ValuesMutIter<V>
where V: Serialize
{
    type Item = DaoMut<V>;

    fn next(&mut self) -> Option<Self::Item>
    {
        self.values.pop()
    }
}

pub struct Iter<'a, K, V>
where V: Serialize
{
    values: Vec<(&'a K, Dao<V>)>
}

impl<'a, K, V> Iterator
for Iter<'a, K, V>
where V: Serialize
{
    type Item = (&'a K, Dao<V>);

    fn next(&mut self) -> Option<Self::Item>
    {
        self.values.pop()
    }
}

pub struct IterMut<'a, K, V>
where V: Serialize
{
    values: Vec<(&'a K, DaoMut<V>)>
}

impl<'a, K, V> Iterator
for IterMut<'a, K, V>
where V: Serialize
{
    type Item = (&'a K, DaoMut<V>);

    fn next(&mut self) -> Option<Self::Item>
    {
        self.values.pop()
    }
}

pub struct Drain<K, V>
where V: Serialize
{
    values: Vec<(K, Dao<V>)>
}

impl<K, V> Iterator
for Drain<K, V>
where V: Serialize
{
    type Item = (K, Dao<V>);

    fn next(&mut self) -> Option<Self::Item>
    {
        self.values.pop()
    }
}