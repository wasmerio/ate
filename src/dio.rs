use serde::{Serialize, de::DeserializeOwned};

use async_trait::async_trait;

use super::header::*;
#[allow(unused_imports)]
use super::chain::*;
#[allow(unused_imports)]
use super::validator::*;
#[allow(unused_imports)]
use super::index::*;
#[allow(unused_imports)]
use super::compact::*;

#[derive(Debug, Clone)]
pub struct Dao<M, D>
    where M: Serialize + DeserializeOwned + Clone,
          D: Serialize + DeserializeOwned + Clone
{
    pub header: Header<M>,
    pub data: D,
}

#[async_trait]
pub trait Dio<M, D>
    where M: MetadataTrait,
          D: Serialize + DeserializeOwned + Clone,
{
    async fn read(key: &PrimaryKey) -> D;
}

/*
pub struct ChainDio<'a, M>
    where M: MetadataTrait,
        V: EventValidator<M> + Default,
        I: EventIndexer<M> + Default,
        C: EventCompactor<M, Index=I> + Default,
{
    chain: Arc<RwLock<ChainOfTrust<M, V, I, C>>>
    chain: &'a mut ChainOfTrust<M, V, I, C>,
}

impl<'a, M, V, I, C> ChainDio<'a, M, V, I, C>
    where M: MetadataTrait,
          V: EventValidator<M> + Default,
          I: EventIndexer<M> + Default,
          C: EventCompactor<M, Index=I> + Default,
{
    pub fn new(chain: &'a mut ChainOfTrust<M, V, I, C>) -> ChainDio<'a, M, V, I, C> {
        ChainDio {
            chain: chain,
        }
    }
}

#[async_trait]
impl<'a, M, D> Dio<M, D> for ChainDio
    where M: MetadataTrait,
{

}
*/