use std::marker::PhantomData;

use serde::*;
use serde::de::*;
use crate::meta::*;
use crate::dio::*;
use crate::dio::dao::*;
use crate::error::*;
use std::collections::VecDeque;

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
#[derive(Debug, Serialize, Deserialize)]
pub struct DaoVec<D>
{
    pub(super) vec_id: u64,
    #[serde(skip)]
    _phantom1: PhantomData<D>,
}

impl<D> Clone
for DaoVec<D>
{
    fn clone(&self) -> DaoVec<D>
    {
        DaoVec {
            vec_id: self.vec_id,
            _phantom1: PhantomData,
        }
    }
}

impl<D> Copy
for DaoVec<D> { }

impl<D> DaoVec<D>
{
    pub fn new() -> DaoVec<D> {
        DaoVec {
            vec_id: fastrand::u64(..),
            _phantom1: PhantomData,
        }
    }
}

impl<D> Default
for DaoVec<D>
{
    fn default() -> DaoVec<D>
    {
        DaoVec::new()
    }
}

impl<D> Dao<D>
where D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    #[allow(dead_code)]
    pub async fn iter<'a, C>(&self, dio: &mut Dio<'a>, vec: DaoVec<C>) -> Result<Iter<C>, LoadError>
    where C: Serialize + DeserializeOwned + Clone + Send + Sync
    {
        Ok(
            Iter::new(
                dio.children(self.key().clone(), vec.vec_id).await?
            )
        )
    }
    
    #[allow(dead_code)]
    pub fn push<C>(&mut self, dio: &mut Dio, vec: DaoVec<C>, data: C) -> Result<Dao<C>, SerializationError>
    where C: Serialize + DeserializeOwned + Clone + Send + Sync
    {
        if self.is_dirty() {
            self.commit(dio)?;
        }
        let mut ret = dio.store_ext(data, None, None, false)?;
        ret.attach(self, vec);
        ret.commit(dio)?;
        Ok (ret)
    }
}

pub struct Iter<D>
where D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    vec: VecDeque<Dao<D>>,
}

impl<D> Iter<D>
where D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    fn new(vec: Vec<Dao<D>>) -> Iter<D> {
        Iter {
            vec: VecDeque::from(vec),
        }
    }
}

impl<D> Iterator
for Iter<D>
where D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    type Item = Dao<D>;

    fn next(&mut self) -> Option<Dao<D>> {
        self.vec.pop_front()
    }
}