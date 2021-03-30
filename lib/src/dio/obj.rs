use std::marker::PhantomData;

use serde::*;
use serde::de::*;
use crate::meta::*;
use crate::dio::*;
use crate::dio::dao::*;
use crate::error::*;
use std::collections::VecDeque;

/// Rerepresents a reference to another data object with strong
/// type linting to make the model more solidified
///
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DaoRef<D>
where D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    pub(super) id: Option<PrimaryKey>,
    #[serde(skip)]
    _phantom1: PhantomData<D>,
}

impl<D> DaoRef<D>
where D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    pub fn new() -> DaoRef<D> {
        DaoRef {
            id: None,
            _phantom1: PhantomData,
        }
    }

    pub fn get_id(&self) -> Option<PrimaryKey> {
        self.id
    }

    pub fn set_id(&mut self, val: PrimaryKey) {
        self.id = Some(val)
    }

    pub fn clear(&mut self) {
        self.id = None;
    }

    /// Loads the data object (if it exists)
    pub async fn load<'a>(&self, dio: &mut Dio<'a>) -> Result<Option<Dao<D>>, LoadError>
    where D: Serialize + DeserializeOwned + Clone + Send + Sync
    {
        let id = match self.id {
            Some(a) => a,
            None => {
                return Ok(None);
            }
        };

        Ok(Some(dio.load::<D>(&id).await?))
    }

    /// Stores the data within this reference
    pub async fn store<'a>(&mut self, dio: &mut Dio<'a>, value: D) -> Result<Dao<D>, LoadError>
    {
        let ret = dio.store::<D>(value)?;
        self.id = Some(ret.key().clone());
        Ok(ret)
    }

    /// Loads the data object or uses a default if none exists
    pub async fn unwrap_or<'a>(&mut self, dio: &mut Dio<'a>, default: D) -> Result<Dao<D>, LoadError>
    where D: Serialize + DeserializeOwned + Clone + Send + Sync
    {
        match self.id {
            Some(id) => {
                Ok(dio.load(&id).await?)
            },
            None => {
                Ok(self.store(dio, default).await?)
            }
        }
    }

    /// Loads the data object or uses a default if none exists
    pub async fn unwrap_or_else<'a, F: FnOnce() -> D>(&mut self, dio: &mut Dio<'a>, f: F) -> Result<Dao<D>, LoadError>
    where D: Serialize + DeserializeOwned + Clone + Send + Sync
    {
        match self.id {
            Some(id) => {
                Ok(dio.load(&id).await?)
            },
            None => {
                Ok(self.store(dio, f()).await?)
            }
        }
    }

    /// Loads the data object or creates a new one (if it does not exist)
    pub async fn unwrap_or_default<'a>(&mut self, dio: &mut Dio<'a>) -> Result<Dao<D>, LoadError>
    where D: Default + Serialize + DeserializeOwned + Clone + Send + Sync
    {
        Ok(self.unwrap_or_else(dio, || {
            let ret: D = Default::default();
            ret
        }).await?)
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
        self.id = None;
        Ok(ret)
    }

    pub async fn replace<'a>(&mut self, dio: &mut Dio<'a>, value: D) -> Result<Option<Dao<D>>, LoadError> {
        let ret = self.load(dio).await?;
        self.store(dio, value).await?;
        Ok(ret)
    }

    pub fn is_some(&self) -> bool {
        self.id.is_some()
    }

    pub fn is_none(&self) -> bool {
        !self.is_some()
    }
}

impl<D> From<PrimaryKey>
for DaoRef<D>
where D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    fn from(key: PrimaryKey) -> DaoRef<D> {
        DaoRef {
            id: Some(key),
            _phantom1: PhantomData
        }
    }
}

impl<D> Default
for DaoRef<D>
where D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    fn default() -> DaoRef<D>
    {
        DaoRef::new()
    }
}