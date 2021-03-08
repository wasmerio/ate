use std::marker::PhantomData;

use serde::*;
use serde::de::*;
use super::meta::*;
use super::dio::*;
use super::dao::*;
use super::error::*;
use std::collections::VecDeque;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DaoVec<D>
where D: Serialize + DeserializeOwned + Clone,
{
    pub(super) vec_id: u64,
    #[serde(skip)]
    _phantom1: PhantomData<D>,
}

impl<D> DaoVec<D>
where D: Serialize + DeserializeOwned + Clone,
{
    pub fn new() -> DaoVec<D> {
        DaoVec {
            vec_id: fastrand::u64(..),
            _phantom1: PhantomData,
        }
    }

    #[allow(dead_code)]
    pub async fn iter<'a, P>(&self, parent: &Dao<P>, dio: &mut Dio<'a>) -> Result<Iter<D>, LoadError>
    where P: Serialize + DeserializeOwned + Clone,
    {
        Ok(
            Iter::new(
                dio.children(parent.key().clone(), self.vec_id.clone()).await?
            )
        )
    }
    
    #[allow(dead_code)]
    pub fn push<P>(&self, dio: &mut Dio, parent: &Dao<P>, data: D) -> Result<Dao<D>, SerializationError>
    where P: Serialize + DeserializeOwned + Clone,
    {
        let mut ret = dio.store(data)?;

        ret.fork();
        ret.row.tree = Some(
            MetaTree {
                vec: MetaCollection {
                    parent_id: parent.key().clone(),
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
where D: Serialize + DeserializeOwned + Clone,
{
    fn default() -> DaoVec<D>
    {
        DaoVec::new()
    }
}
pub struct Iter<D>
where D: Serialize + DeserializeOwned + Clone,
{
    vec: VecDeque<Dao<D>>,
}

impl<D> Iter<D>
where D: Serialize + DeserializeOwned + Clone,
{
    fn new(vec: Vec<Dao<D>>) -> Iter<D> {
        Iter {
            vec: VecDeque::from(vec),
        }
    }
}

impl<D> Iterator
for Iter<D>
where D: Serialize + DeserializeOwned + Clone,
{
    type Item = Dao<D>;

    fn next(&mut self) -> Option<Dao<D>> {
        self.vec.pop_front()
    }
}