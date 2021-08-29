#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use tracing_futures::Instrument;
use error_chain::bail;
use std::marker::PhantomData;
use std::sync::{Arc, Weak};

use serde::*;
use serde::de::*;
use super::dio::DioWeak;
use super::dio_mut::DioMutWeak;
use crate::dio::*;
use crate::dio::dao::*;
use crate::error::*;
use std::collections::VecDeque;
use crate::prelude::*;

/// Rerepresents a vector of children attached to a parent DAO
///
/// This object does not actually store the children which are
/// actually stored within the chain-of-trust as seperate events
/// that are indexed into secondary indexes that this object queries.
///
/// Vectors can also be used as queues and as a bus for various
/// different usecases.
/// 
/// Storing this vector within other DAO's allows complex models
/// to be represented.
///
/// Alternatively you can store your vectors, maps and other
/// relationships as collections of `PrimaryKey`'s however you
/// will need to manage this yourselve and can not benefit from
/// publish/subscribe patterns.
///
#[derive(Serialize, Deserialize)]
pub struct DaoVec<D>
{
    pub(super) vec_id: u64,
    #[serde(skip)]
    pub(super) state: DaoVecState,
    #[serde(skip)]
    pub(super) dio: DioWeak,
    #[serde(skip)]
    pub(super) dio_mut: DioMutWeak,
    #[serde(skip)]
    pub(super) _phantom1: PhantomData<D>,
}

pub(super) enum DaoVecState
{
    Unsaved,
    Saved(PrimaryKey)
}

impl Default
for DaoVecState
{
    fn default() -> Self
    {
        match PrimaryKey::current_get() {
            Some(a) => DaoVecState::Saved(a),
            None => DaoVecState::Unsaved
        }
    }
}

impl Clone
for DaoVecState
{
    fn clone(&self) -> Self
    {
        match self {
            Self::Unsaved => Self::default(),
            Self::Saved(a) => Self::Saved(a.clone())
        }
    }
}

impl<D> std::fmt::Debug
for DaoVec<D>
where D: std::fmt::Debug
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let type_name = std::any::type_name::<D>();
        write!(f, "dao-vec(vec_id={}, type={}", self.vec_id, type_name)
    }
}

impl<D> Default
for DaoVec<D>
{
    fn default() -> Self {
        DaoVec::new()
    }
}

impl<D> Clone
for DaoVec<D>
{
    fn clone(&self) -> DaoVec<D>
    {
        DaoVec {
            state: self.state.clone(),
            vec_id: self.vec_id,
            dio: self.dio.clone(),
            dio_mut: self.dio_mut.clone(),
            _phantom1: PhantomData,
        }
    }
}

impl<D> DaoVec<D>
{
    pub fn new() -> DaoVec<D> {
        DaoVec {
            state: DaoVecState::Unsaved,
            dio: DioWeak::Uninitialized,
            dio_mut: DioMutWeak::Uninitialized,
            vec_id: fastrand::u64(..),
            _phantom1: PhantomData,
        }
    }
    
    pub fn new_orphaned(dio: &Arc<Dio>, parent: PrimaryKey, vec_id: u64) -> DaoVec<D> {
        DaoVec {
            state: DaoVecState::Saved(parent),
            dio: DioWeak::from(dio),
            dio_mut: DioMutWeak::Uninitialized,
            vec_id: vec_id,
            _phantom1: PhantomData,
        }
    }

    pub fn new_orphaned_mut(dio: &Arc<DioMut>, parent: PrimaryKey, vec_id: u64) -> DaoVec<D> {
        DaoVec {
            state: DaoVecState::Saved(parent),
            dio: DioWeak::from(&dio.dio),
            dio_mut: DioMutWeak::from(dio),
            vec_id: vec_id,
            _phantom1: PhantomData,
        }
    }

    pub fn dio(&self) -> Option<Arc<Dio>> {
        match &self.dio {
            DioWeak::Uninitialized => None,
            DioWeak::Weak(a) => Weak::upgrade(a)
        }
    }

    pub fn dio_mut(&self) -> Option<Arc<DioMut>> {
        match &self.dio_mut {
            DioMutWeak::Uninitialized => None,
            DioMutWeak::Weak(a) => Weak::upgrade(a)
        }
    }

    pub fn vec_id(&self) -> u64 {
        self.vec_id
    }

    pub async fn len(&self) -> Result<usize, LoadError>
    {
        let len = match &self.state {
            DaoVecState::Unsaved => 0usize,
            DaoVecState::Saved(parent_id) =>
            {
                let dio = match self.dio() {
                    Some(a) => a,
                    None => bail!(LoadErrorKind::WeakDio)
                };
                
                dio.children_keys(parent_id.clone(), self.vec_id).await?.len()
            },
        };
        Ok(len)
    }

    pub async fn iter(&self) -> Result<Iter<D>, LoadError>
    where D: Serialize + DeserializeOwned
    {
        self.iter_ext(false, false).await
    }

    pub async fn iter_ext(&self, allow_missing_keys: bool, allow_serialization_error: bool) -> Result<Iter<D>, LoadError>
    where D: Serialize + DeserializeOwned
    {
        let children = match &self.state {
            DaoVecState::Unsaved => vec![],
            DaoVecState::Saved(parent_id) =>
            {
                if let Some(dio) = self.dio_mut() {
                    dio.children_ext(parent_id.clone(), self.vec_id, allow_missing_keys, allow_serialization_error).await?
                        .into_iter()
                        .map(|a: DaoMut<D>| a.inner)
                        .collect::<Vec<_>>()
                } else {
                    let dio = match self.dio() {
                        Some(a) => a,
                        None => bail!(LoadErrorKind::WeakDio)
                    };
                    
                    dio.children_ext(parent_id.clone(), self.vec_id, allow_missing_keys, allow_serialization_error).await?
                }
            },
        };

        Ok(
            Iter::new(
            children                
            )
        )
    }

    pub async fn iter_mut(&mut self) -> Result<IterMut<D>, LoadError>
    where D: Serialize + DeserializeOwned
    {
        self.iter_mut_ext(false, false).await
    }

    pub async fn iter_mut_ext(&mut self, allow_missing_keys: bool, allow_serialization_error: bool) -> Result<IterMut<D>, LoadError>
    where D: Serialize + DeserializeOwned
    {
        let children = match &self.state {
            DaoVecState::Unsaved => vec![],
            DaoVecState::Saved(parent_id) =>
            {
                let dio = match self.dio_mut() {
                    Some(a) => a,
                    None => bail!(LoadErrorKind::WeakDio)
                };
                
                let mut ret = Vec::default();
                for child in dio.children_ext::<D>(parent_id.clone(), self.vec_id, allow_missing_keys, allow_serialization_error).await? {
                    ret.push(child)
                }
                ret
            },
        };

        Ok(
            IterMut::new(
            children                
            )
        )
    }
    
    pub async fn iter_mut_with_dio(&self, dio: &Arc<DioMut>) -> Result<IterMut<D>, LoadError>
    where D: Serialize + DeserializeOwned
    {
        self.iter_mut_ext_with_dio(dio, false, false).await
    }

    pub async fn iter_mut_ext_with_dio(&self, dio: &Arc<DioMut>, allow_missing_keys: bool, allow_serialization_error: bool) -> Result<IterMut<D>, LoadError>
    where D: Serialize + DeserializeOwned
    {
        let children = match &self.state {
            DaoVecState::Unsaved => vec![],
            DaoVecState::Saved(parent_id) =>
            {
                let mut ret = Vec::default();
                for child in dio.children_ext::<D>(parent_id.clone(), self.vec_id, allow_missing_keys, allow_serialization_error).await? {
                    ret.push(child)
                }
                ret
            },
        };

        Ok(
            IterMut::new(
            children
            )
        )
    }

    pub fn push(&mut self, data: D) -> Result<DaoMut<D>, SerializationError>
    where D: Clone + Serialize + DeserializeOwned,
    {
        let dio = match self.dio_mut() {
            Some(a) => a,
            None => bail!(SerializationErrorKind::WeakDio)
        };

        let parent_id = match &self.state {
            DaoVecState::Unsaved => { bail!(SerializationErrorKind::SaveParentFirst); },
            DaoVecState::Saved(a) => a.clone(),
        };

        let mut ret = dio.store(data)?;
        ret.attach_ext(parent_id, self.vec_id)?;
        Ok(ret)
    }

    pub fn push_with_key(&mut self, data: D, key: PrimaryKey) -> Result<DaoMut<D>, SerializationError>
    where D: Clone + Serialize + DeserializeOwned,
    {
        let dio = match self.dio_mut() {
            Some(a) => a,
            None => bail!(SerializationErrorKind::WeakDio)
        };

        let parent_id = match &self.state {
            DaoVecState::Unsaved => { bail!(SerializationErrorKind::SaveParentFirst); },
            DaoVecState::Saved(a) => a.clone(),
        };

        let mut ret = dio.store_with_key(data, key)?;
        ret.attach_ext(parent_id, self.vec_id)?;
        Ok(ret)
    }

    pub fn push_with_dio(&self, dio: &Arc<DioMut>, data: D) -> Result<DaoMut<D>, SerializationError>
    where D: Clone + Serialize + DeserializeOwned,
    {
        let parent_id = match &self.state {
            DaoVecState::Unsaved => { bail!(SerializationErrorKind::SaveParentFirst); },
            DaoVecState::Saved(a) => a.clone(),
        };

        let mut ret = dio.store(data)?;
        ret.attach_ext(parent_id, self.vec_id)?;
        Ok(ret)
    }

    pub fn push_with_dio_and_key(&self, dio: &Arc<DioMut>, data: D, key: PrimaryKey) -> Result<DaoMut<D>, SerializationError>
    where D: Clone + Serialize + DeserializeOwned,
    {
        let parent_id = match &self.state {
            DaoVecState::Unsaved => { bail!(SerializationErrorKind::SaveParentFirst); },
            DaoVecState::Saved(a) => a.clone(),
        };

        let mut ret = dio.store_with_key(data, key)?;
        ret.attach_ext(parent_id, self.vec_id)?;
        Ok(ret)
    }
}

pub struct Iter<D>
{
    vec: VecDeque<Dao<D>>,
}

impl<D> Iter<D>
{
    pub(super) fn new(vec: Vec<Dao<D>>) -> Iter<D> {
        Iter {
            vec: VecDeque::from(vec),
        }
    }
}

impl<D> Iterator
for Iter<D>
{
    type Item = Dao<D>;

    fn next(&mut self) -> Option<Dao<D>> {
        self.vec.pop_front()
    }
}

pub struct IterMut<D>
where D: Serialize
{
    vec: VecDeque<DaoMut<D>>,
}

impl<D> IterMut<D>
where D: Serialize
{
    pub(super) fn new(vec: Vec<DaoMut<D>>) -> IterMut<D> {
        IterMut {
            vec: VecDeque::from(vec),
        }
    }
}

impl<D> Iterator
for IterMut<D>
where D: Serialize
{
    type Item = DaoMut<D>;

    fn next(&mut self) -> Option<DaoMut<D>> {
        self.vec.pop_front()
    }
}