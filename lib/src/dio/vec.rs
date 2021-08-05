#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use tracing_futures::Instrument;
use error_chain::bail;
use std::marker::PhantomData;
use std::sync::{Arc, Weak};

use serde::*;
use serde::de::*;
use super::dio::DioWeak;
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
    dio: DioWeak,
    #[serde(skip)]
    _phantom1: PhantomData<D>,
}

#[derive(Clone)]
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
            vec_id: fastrand::u64(..),
            _phantom1: PhantomData,
        }
    }
}

impl<D> DaoVec<D>
{
    pub fn dio(&self) -> Option<Arc<Dio>> {
        match &self.dio {
            DioWeak::Uninitialized => None,
            DioWeak::Weak(a) => Weak::upgrade(a)
        }
    }

    pub async fn iter(&self) -> Result<Iter<D>, LoadError>
    where D: DeserializeOwned
    {
        self.iter_ext(false, false).await
    }

    pub async fn iter_ext(&self, allow_missing_keys: bool, allow_serialization_error: bool) -> Result<Iter<D>, LoadError>
    where D: DeserializeOwned
    {
        let children = match &self.state {
            DaoVecState::Unsaved => vec![],
            DaoVecState::Saved(parent_id) =>
            {
                let dio = match self.dio() {
                    Some(a) => a,
                    None => return Err(LoadError::WeakDio)
                };
                
                dio.children_ext(parent_id.clone(), self.vec_id, allow_missing_keys, allow_serialization_error).await?
            },
        };

        Ok(
            Iter::new(
            children                
            )
        )
    }

    pub fn push(&self, trans: &Arc<DioMut>, data: D) -> Result<DaoMut<D>, SerializationError>
    where D: Serialize + DeserializeOwned,
    {
        let parent_id = match &self.state {
            DaoVecState::Unsaved => { return Err(SerializationError::SaveParentFirst); },
            DaoVecState::Saved(a) => a.clone(),
        };

        let mut ret = trans.store(data)?;
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
    fn new(vec: Vec<Dao<D>>) -> Iter<D> {
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