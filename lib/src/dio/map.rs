#![allow(unused_imports)]
use error_chain::bail;
use fxhash::FxHashMap;
use std::marker::PhantomData;
use std::sync::{Arc, Weak};
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use tracing_futures::Instrument;

use super::dio::DioWeak;
use super::dio_mut::DioMutWeak;
use super::vec::DaoVecState;
use crate::dio::dao::*;
use crate::dio::*;
use crate::error::*;
use crate::prelude::*;
use serde::de::*;
use serde::*;
use std::collections::VecDeque;

#[derive(Serialize, Deserialize)]
pub struct DaoMap<K, V> {
    pub(super) lookup: FxHashMap<String, PrimaryKey>,
    pub(super) vec_id: u64,
    #[serde(skip)]
    pub(super) state: DaoMapState,
    #[serde(skip)]
    dio: DioWeak,
    #[serde(skip)]
    dio_mut: DioMutWeak,
    #[serde(skip)]
    _phantom1: PhantomData<K>,
    #[serde(skip)]
    _phantom2: PhantomData<V>,
}

pub(super) enum DaoMapState {
    Unsaved,
    Saved(PrimaryKey),
}

impl Default for DaoMapState {
    fn default() -> Self {
        match PrimaryKey::current_get() {
            Some(a) => DaoMapState::Saved(a),
            None => DaoMapState::Unsaved,
        }
    }
}

impl Clone for DaoMapState {
    fn clone(&self) -> Self {
        match self {
            Self::Unsaved => Self::default(),
            Self::Saved(a) => Self::Saved(a.clone()),
        }
    }
}

impl<K, V> std::fmt::Debug for DaoMap<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let key_type_name = std::any::type_name::<K>();
        let value_type_name = std::any::type_name::<V>();
        write!(
            f,
            "dao-map(vec_id={}, key-type={}, value-type={}",
            self.vec_id, key_type_name, value_type_name
        )
    }
}

impl<K, V> Default for DaoMap<K, V> {
    fn default() -> Self {
        DaoMap::new()
    }
}

impl<K, V> Clone for DaoMap<K, V> {
    fn clone(&self) -> DaoMap<K, V> {
        DaoMap {
            lookup: self.lookup.clone(),
            state: self.state.clone(),
            vec_id: self.vec_id,
            dio: self.dio.clone(),
            dio_mut: self.dio_mut.clone(),
            _phantom1: PhantomData,
            _phantom2: PhantomData,
        }
    }
}

impl<K, V> DaoMap<K, V> {
    pub fn new() -> DaoMap<K, V> {
        DaoMap {
            lookup: FxHashMap::default(),
            state: DaoMapState::Unsaved,
            dio: DioWeak::Uninitialized,
            dio_mut: DioMutWeak::Uninitialized,
            vec_id: fastrand::u64(..),
            _phantom1: PhantomData,
            _phantom2: PhantomData,
        }
    }
}

impl<K, V> DaoMap<K, V> {
    pub fn new_orphaned(dio: &Arc<Dio>, parent: PrimaryKey, vec_id: u64) -> DaoMap<K, V> {
        DaoMap {
            lookup: FxHashMap::default(),
            state: DaoMapState::Saved(parent),
            dio: DioWeak::from(dio),
            dio_mut: DioMutWeak::Uninitialized,
            vec_id: vec_id,
            _phantom1: PhantomData,
            _phantom2: PhantomData,
        }
    }

    pub fn new_orphaned_mut(dio: &Arc<DioMut>, parent: PrimaryKey, vec_id: u64) -> DaoMap<K, V> {
        DaoMap {
            lookup: FxHashMap::default(),
            state: DaoMapState::Saved(parent),
            dio: DioWeak::from(&dio.dio),
            dio_mut: DioMutWeak::from(dio),
            vec_id: vec_id,
            _phantom1: PhantomData,
            _phantom2: PhantomData,
        }
    }

    pub fn dio(&self) -> Option<Arc<Dio>> {
        match &self.dio {
            DioWeak::Uninitialized => None,
            DioWeak::Weak(a) => Weak::upgrade(a),
        }
    }

    pub fn dio_mut(&self) -> Option<Arc<DioMut>> {
        match &self.dio_mut {
            DioMutWeak::Uninitialized => None,
            DioMutWeak::Weak(a) => Weak::upgrade(a),
        }
    }

    pub fn as_vec(&self) -> DaoVec<V> {
        DaoVec {
            vec_id: self.vec_id,
            state: match &self.state {
                DaoMapState::Saved(a) => DaoVecState::Saved(a.clone()),
                DaoMapState::Unsaved => DaoVecState::Unsaved,
            },
            dio: self.dio.clone(),
            dio_mut: self.dio_mut.clone(),
            _phantom1: PhantomData,
        }
    }

    pub fn vec_id(&self) -> u64 {
        self.vec_id
    }

    pub async fn len(&self) -> Result<usize, LoadError> {
        let len = match &self.state {
            DaoMapState::Unsaved => self.lookup.len(),
            DaoMapState::Saved(parent_id) => {
                let dio = match self.dio() {
                    Some(a) => a,
                    None => bail!(LoadErrorKind::WeakDio),
                };

                dio.children_keys(parent_id.clone(), self.vec_id)
                    .await?
                    .len()
            }
        };
        Ok(len)
    }

    pub async fn iter(&self) -> Result<Iter<K, V>, LoadError>
    where
        K: DeserializeOwned,
        V: Serialize + DeserializeOwned,
    {
        self.iter_ext(false, false).await
    }

    pub async fn iter_ext(
        &self,
        allow_missing_keys: bool,
        allow_serialization_error: bool,
    ) -> Result<Iter<K, V>, LoadError>
    where
        K: DeserializeOwned,
        V: Serialize + DeserializeOwned,
    {
        let mut reverse = FxHashMap::default();
        for (k, v) in self.lookup.iter() {
            reverse.insert(v, k);
        }

        let children = match &self.state {
            DaoMapState::Unsaved => vec![],
            DaoMapState::Saved(parent_id) => {
                if let Some(dio) = self.dio_mut() {
                    dio.children_ext(
                        parent_id.clone(),
                        self.vec_id,
                        allow_missing_keys,
                        allow_serialization_error,
                    )
                    .await?
                    .into_iter()
                    .map(|a: DaoMut<V>| a.inner)
                    .collect::<Vec<_>>()
                } else {
                    let dio = match self.dio() {
                        Some(a) => a,
                        None => bail!(LoadErrorKind::WeakDio),
                    };

                    dio.children_ext(
                        parent_id.clone(),
                        self.vec_id,
                        allow_missing_keys,
                        allow_serialization_error,
                    )
                    .await?
                }
            }
        };

        let pairs = children
            .into_iter()
            .filter_map(|v| match reverse.get(v.key()) {
                Some(k) => {
                    let k = base64::decode(k)
                        .ok()
                        .map(|a| bincode::deserialize(&a[..]).ok())
                        .flatten();
                    match k {
                        Some(k) => Some((k, v)),
                        None => None,
                    }
                }
                None => None,
            })
            .collect::<Vec<_>>();

        Ok(Iter::new(pairs))
    }

    pub async fn iter_mut(&mut self) -> Result<IterMut<K, V>, LoadError>
    where
        K: DeserializeOwned,
        V: Serialize + DeserializeOwned,
    {
        self.iter_mut_ext(false, false).await
    }

    pub async fn iter_mut_ext(
        &mut self,
        allow_missing_keys: bool,
        allow_serialization_error: bool,
    ) -> Result<IterMut<K, V>, LoadError>
    where
        K: DeserializeOwned,
        V: Serialize + DeserializeOwned,
    {
        let dio = match self.dio_mut() {
            Some(a) => a,
            None => bail!(LoadErrorKind::WeakDio),
        };

        self.iter_mut_ext_with_dio(&dio, allow_missing_keys, allow_serialization_error)
            .await
    }

    pub async fn iter_mut_with_dio(&self, dio: &Arc<DioMut>) -> Result<IterMut<K, V>, LoadError>
    where
        K: DeserializeOwned,
        V: Serialize + DeserializeOwned,
    {
        self.iter_mut_ext_with_dio(dio, false, false).await
    }

    pub async fn iter_mut_ext_with_dio(
        &self,
        dio: &Arc<DioMut>,
        allow_missing_keys: bool,
        allow_serialization_error: bool,
    ) -> Result<IterMut<K, V>, LoadError>
    where
        K: DeserializeOwned,
        V: Serialize + DeserializeOwned,
    {
        let mut reverse = FxHashMap::default();
        for (k, v) in self.lookup.iter() {
            reverse.insert(v, k);
        }

        let children = match &self.state {
            DaoMapState::Unsaved => vec![],
            DaoMapState::Saved(parent_id) => {
                let mut ret = Vec::default();
                for child in dio
                    .children_ext::<V>(
                        parent_id.clone(),
                        self.vec_id,
                        allow_missing_keys,
                        allow_serialization_error,
                    )
                    .await?
                {
                    ret.push(child)
                }
                ret
            }
        };

        let pairs = children
            .into_iter()
            .filter_map(|v| match reverse.get(v.key()) {
                Some(k) => {
                    let k = base64::decode(k)
                        .ok()
                        .map(|a| bincode::deserialize(&a[..]).ok())
                        .flatten();
                    match k {
                        Some(k) => Some((k, v)),
                        None => None,
                    }
                }
                None => None,
            })
            .collect::<Vec<_>>();

        Ok(IterMut::new(pairs))
    }

    pub async fn insert(&mut self, key: K, value: V) -> Result<(), SerializationError>
    where
        K: Serialize,
        V: Clone + Serialize + DeserializeOwned,
    {
        self.insert_ret(key, value).await?;
        Ok(())
    }

    pub async fn insert_ret(&mut self, key: K, value: V) -> Result<DaoMut<V>, SerializationError>
    where
        K: Serialize,
        V: Clone + Serialize + DeserializeOwned,
    {
        let dio = match self.dio_mut() {
            Some(a) => a,
            None => bail!(SerializationErrorKind::WeakDio),
        };

        let parent_id = match &self.state {
            DaoMapState::Unsaved => {
                bail!(SerializationErrorKind::SaveParentFirst);
            }
            DaoMapState::Saved(a) => a.clone(),
        };

        let key = base64::encode(&bincode::serialize(&key)?[..]);

        let mut ret = dio.store(value)?;
        ret.attach_ext(parent_id, self.vec_id)?;

        if let Some(old) = self.lookup.insert(key, ret.key().clone()) {
            dio.delete(&old).await?;
        }

        Ok(ret)
    }

    pub async fn get(&self, key: &K) -> Result<Option<Dao<V>>, LoadError>
    where
        K: Serialize,
        V: Serialize + DeserializeOwned,
    {
        let key = base64::encode(&bincode::serialize(key)?[..]);

        let id = match self.lookup.get(&key) {
            Some(a) => a,
            None => {
                return Ok(None);
            }
        };

        if let Some(dio) = self.dio_mut() {
            let ret = match dio.load::<V>(&id).await {
                Ok(a) => Some(a.inner),
                Err(LoadError(LoadErrorKind::NotFound(_), _)) => None,
                Err(err) => {
                    bail!(err);
                }
            };
            return Ok(ret);
        }

        let dio = match self.dio() {
            Some(a) => a,
            None => bail!(LoadErrorKind::WeakDio),
        };

        let ret = match dio.load::<V>(&id).await {
            Ok(a) => Some(a),
            Err(LoadError(LoadErrorKind::NotFound(_), _)) => None,
            Err(err) => {
                bail!(err);
            }
        };
        Ok(ret)
    }

    pub async fn get_mut(&mut self, key: &K) -> Result<Option<DaoMut<V>>, LoadError>
    where
        K: Serialize,
        V: Serialize + DeserializeOwned,
    {
        let key = base64::encode(&bincode::serialize(key)?[..]);

        let id = match self.lookup.get(&key) {
            Some(a) => a,
            None => {
                return Ok(None);
            }
        };

        let dio = match self.dio_mut() {
            Some(a) => a,
            None => bail!(LoadErrorKind::WeakDio),
        };

        let ret = match dio.load::<V>(&id).await {
            Ok(a) => Some(a),
            Err(LoadError(LoadErrorKind::NotFound(_), _)) => None,
            Err(err) => {
                bail!(err);
            }
        };
        Ok(ret)
    }

    pub async fn get_or_default(&mut self, key: K) -> Result<DaoMut<V>, LoadError>
    where
        K: Serialize,
        V: Clone + Serialize + DeserializeOwned + Default,
    {
        self.get_or_insert_with(key, || Default::default()).await
    }

    pub async fn get_or_insert(&mut self, key: K, default_val: V) -> Result<DaoMut<V>, LoadError>
    where
        K: Serialize,
        V: Clone + Serialize + DeserializeOwned + Default,
    {
        self.get_or_insert_with(key, || default_val).await
    }

    pub async fn get_or_insert_with<F>(
        &mut self,
        key: K,
        default: F,
    ) -> Result<DaoMut<V>, LoadError>
    where
        F: FnOnce() -> V,
        K: Serialize,
        V: Clone + Serialize + DeserializeOwned,
    {
        let key = base64::encode(&bincode::serialize(&key)?[..]);
        let id = self.lookup.entry(key).or_default().clone();

        let dio = match self.dio_mut() {
            Some(a) => a,
            None => bail!(LoadErrorKind::WeakDio),
        };

        let ret = match dio.load::<V>(&id).await {
            Ok(a) => a,
            Err(LoadError(LoadErrorKind::NotFound(_), _)) => {
                let parent_id = match &self.state {
                    DaoMapState::Unsaved => {
                        bail!(LoadErrorKind::SerializationError(
                            SerializationErrorKind::SaveParentFirst
                        ));
                    }
                    DaoMapState::Saved(a) => a.clone(),
                };

                let mut ret = dio.store_with_key(default(), id)?;
                ret.attach_ext(parent_id, self.vec_id)?;
                ret
            }
            Err(err) => {
                bail!(err);
            }
        };
        Ok(ret)
    }

    pub async fn delete(&mut self, key: &K) -> Result<bool, SerializationError>
    where
        K: Serialize,
        V: Serialize,
    {
        let key = base64::encode(&bincode::serialize(key)?[..]);

        let id = match self.lookup.get(&key) {
            Some(a) => a,
            None => {
                return Ok(false);
            }
        };

        let dio = match self.dio_mut() {
            Some(a) => a,
            None => bail!(SerializationErrorKind::WeakDio),
        };

        if dio.exists(&id).await == false {
            return Ok(false);
        }

        dio.delete(&id).await?;
        Ok(true)
    }
}

pub struct Iter<K, V> {
    vec: VecDeque<(K, Dao<V>)>,
}

impl<K, V> Iter<K, V> {
    pub(super) fn new(vec: Vec<(K, Dao<V>)>) -> Iter<K, V> {
        Iter {
            vec: VecDeque::from(vec),
        }
    }
}

impl<K, V> Iterator for Iter<K, V> {
    type Item = (K, Dao<V>);

    fn next(&mut self) -> Option<(K, Dao<V>)> {
        self.vec.pop_front()
    }
}

pub struct IterMut<K, V>
where
    V: Serialize,
{
    vec: VecDeque<(K, DaoMut<V>)>,
}

impl<K, V> IterMut<K, V>
where
    V: Serialize,
{
    pub(super) fn new(vec: Vec<(K, DaoMut<V>)>) -> IterMut<K, V> {
        IterMut {
            vec: VecDeque::from(vec),
        }
    }
}

impl<K, V> Iterator for IterMut<K, V>
where
    V: Serialize,
{
    type Item = (K, DaoMut<V>);

    fn next(&mut self) -> Option<(K, DaoMut<V>)> {
        self.vec.pop_front()
    }
}
