use std::marker::PhantomData;

use serde::*;
use serde::de::*;
use super::meta::*;
use super::dio::*;
use super::error::*;
use std::collections::VecDeque;

#[allow(dead_code)]
pub type DaoVec<D> = DaoVecExt<NoAdditionalMetadata, D>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DaoVecExt<M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
{
    pub(super) vec_id: u64,
    #[serde(skip)]
    _phantom1: PhantomData<M>,
    #[serde(skip)]
    _phantom2: PhantomData<D>,
}

impl<M, D> DaoVecExt<M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
{
    pub fn new() -> DaoVecExt<M, D> {
        DaoVecExt {
            vec_id: fastrand::u64(..),
            _phantom1: PhantomData,
            _phantom2: PhantomData,
        }
    }

    #[allow(dead_code)]
    pub fn iter<P>(&self, parent: &DaoExt<M, P>, dio: &mut DioExt<M>) -> Result<Iter<M, D>, LoadError>
    where P: Serialize + DeserializeOwned + Clone,
    {
        Ok(
            Iter::new(
                dio.children(parent.key().clone(), self.vec_id.clone())?
            )
        )
    }
    
    #[allow(dead_code)]
    pub fn push<P>(&self, dio: &mut DioExt<M>, parent: &DaoExt<M, P>, data: D) -> Result<DaoExt<M, D>, SerializationError>
    where P: Serialize + DeserializeOwned + Clone,
    {
        let mut ret = dio.store(data)?;

        ret.fork();
        ret.row.tree = Some(
            MetaTree {
                parent_id: parent.key().clone(),
                collection_id: self.vec_id.clone(),
                inherit_read: true,
                inherit_write: true,
            }
        );
        Ok (ret)
    }
}

impl<M, D> Default
for DaoVecExt<M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
{
    fn default() -> DaoVecExt<M, D>
    {
        DaoVecExt::new()
    }
}
pub struct Iter<M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
{
    vec: VecDeque<DaoExt<M, D>>,
}

impl<M, D> Iter<M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
{
    fn new(vec: Vec<DaoExt<M, D>>) -> Iter<M, D> {
        Iter {
            vec: VecDeque::from(vec),
        }
    }
}

impl<M, D> Iterator
for Iter<M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
{
    type Item = DaoExt<M, D>;

    fn next(&mut self) -> Option<DaoExt<M, D>> {
        self.vec.pop_front()
    }
}