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
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DaoVec<D>
where D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    pub(super) vec_id: u64,
    #[serde(skip)]
    _phantom1: PhantomData<D>,
}

impl<D> DaoVec<D>
where D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    pub fn new() -> DaoVec<D> {
        DaoVec {
            vec_id: fastrand::u64(..),
            _phantom1: PhantomData,
        }
    }

    #[allow(dead_code)]
    pub async fn iter<'a>(&self, parent_id: &PrimaryKey, dio: &mut Dio<'a>) -> Result<Iter<D>, LoadError>
    {
        Ok(
            Iter::new(
                dio.children(parent_id.clone(), self.vec_id.clone()).await?
            )
        )
    }
    
    #[allow(dead_code)]
    pub fn push(&self, dio: &mut Dio, parent_id: &PrimaryKey, data: D) -> Result<Dao<D>, SerializationError>
    {
        let mut ret = dio.store_ext(data, None, None, false)?;
        ret.attach(parent_id, self);
        ret.commit(dio)?;
        Ok (ret)
    }
}

impl<D> Default
for DaoVec<D>
where D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    fn default() -> DaoVec<D>
    {
        DaoVec::new()
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