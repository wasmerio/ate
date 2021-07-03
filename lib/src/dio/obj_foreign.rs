use std::marker::PhantomData;

use serde::*;
use serde::de::*;
use crate::dio::*;
use crate::dio::dao::*;
use crate::error::*;
use crate::header::*;
use crate::chain::ChainKey;

/// Rerepresents a reference to another data object that resides in
/// another chain-of-trust with strong type linting
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DaoRefForeign<D>
where D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    pub(super) target: Option<(ChainKey, PrimaryKey)>,
    #[serde(skip)]
    _phantom1: PhantomData<D>,
}

impl<D> DaoRefForeign<D>
where D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    pub fn new() -> DaoRefForeign<D> {
        DaoRefForeign {
            target: None,
            _phantom1: PhantomData,
        }
    }

    pub fn get_chain(&self) -> Option<ChainKey> {
        self.target.as_ref().map(|a| a.0.clone())
    }

    pub fn get_id(&self) -> Option<PrimaryKey> {
        self.target.as_ref().map(|a| a.1)
    }

    pub fn get_target(&self) -> Option<(ChainKey, PrimaryKey)> {
        self.target.as_ref().map(|(a, b)| (a.clone(), b.clone()))
    }

    pub fn set_target(&mut self, chain: ChainKey, val: PrimaryKey) {
        self.target = Some((chain, val))
    }

    pub fn clear(&mut self) {
        self.target = None;
    }

    /// Loads the data object (if it exists)
    pub async fn load<'a>(&self, dio: &mut Dio<'a>) -> Result<Option<Dao<D>>, LoadError>
    where D: Serialize + DeserializeOwned + Clone + Send + Sync
    {
        let (chain, id) = match self.target.as_ref() {
            Some(a) => a,
            None => {
                return Ok(None);
            }
        };

        let repo = match dio.multi.repository() {
            Some(a) => a,
            None => return Err(LoadError::NoRepository)
        };
        let session = dio.session;

        let chain = repo.open_by_key(&chain).await?;
        let mut dio = chain.dio(session).await;

        Ok(Some(dio.load::<D>(&id).await?))
    }

    /// Stores the data within this reference
    pub async fn store<'a>(&mut self, dio: &mut Dio<'a>, value: D) -> Result<Dao<D>, LoadError>
    {
        let ret = dio.store::<D>(value)?;
        
        let chain = dio.multi.chain.clone();
        let id = ret.key().clone();
        
        self.target = Some((chain, id));
        Ok(ret)
    }

    pub async fn expect<'a>(&mut self, dio: &mut Dio<'a>, msg: &str) -> Dao<D>
    where D: Serialize + DeserializeOwned + Clone + Send + Sync
    {
        match self.load(dio).await {
            Ok(Some(a)) => a,
            Ok(None) => {
                panic!("{}", msg);
            }
            Err(err) => {
                panic!("{}: {:?}", msg, err);
            }
        }
    }

    pub async fn unwrap<'a>(&mut self, dio: &mut Dio<'a>) -> Dao<D>
    {
        self.expect(dio, "called `DaoRef::unwrap()` that failed to load").await
    }

    pub async fn take<'a>(&mut self, dio: &mut Dio<'a>) -> Result<Option<Dao<D>>, LoadError> {
        let ret = self.load(dio).await?;
        self.target = None;
        Ok(ret)
    }

    pub async fn replace<'a>(&mut self, dio: &mut Dio<'a>, value: D) -> Result<Option<Dao<D>>, LoadError> {
        let ret = self.load(dio).await?;
        self.store(dio, value).await?;
        Ok(ret)
    }

    pub fn is_some(&self) -> bool {
        self.target.is_some()
    }

    pub fn is_none(&self) -> bool {
        !self.is_some()
    }
}

impl<D> Default
for DaoRefForeign<D>
where D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    fn default() -> DaoRefForeign<D>
    {
        DaoRefForeign::new()
    }
}