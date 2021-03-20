use std::marker::PhantomData;

use serde::*;
use serde::de::*;
use crate::meta::*;
use crate::dio::*;
use crate::dio::dao::*;
use crate::error::*;
use std::collections::VecDeque;

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
        let mut ret = dio.store(data)?;

        ret.fork();
        ret.row.tree = Some(
            MetaTree {
                vec: MetaCollection {
                    parent_id: parent_id.clone(),
                    collection_id: self.vec_id.clone(),
                },
                inherit_read: true,
                inherit_write: true,
            }
        );
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