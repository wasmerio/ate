#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use tracing_futures::Instrument;
use error_chain::bail;
use std::marker::PhantomData;
use std::sync::{Arc, Weak};

use serde::*;
use serde::de::*;
use crate::dio::*;
use super::dio::DioWeak;
use crate::dio::dao::*;
use crate::error::*;
use crate::header::*;

/// Rerepresents a reference to another data object with strong
/// type linting to make the model more solidified
///
#[derive(Serialize, Deserialize)]
pub struct DaoRef<D>
{
    pub(super) id: Option<PrimaryKey>,
    #[serde(skip)]
    dio: DioWeak,
    #[serde(skip)]
    _marker: PhantomData<D>,
}

impl<D> Clone
for DaoRef<D>
{
    fn clone(&self) -> Self
    {
        DaoRef {
            id: self.id.clone(),
            dio: self.dio.clone(),
            _marker: PhantomData,
        }
    }
}

impl<D> Default
for DaoRef<D>
{
    fn default() -> Self {
        DaoRef::new()
    }
}

impl<D> std::fmt::Debug
for DaoRef<D>
where D: std::fmt::Debug
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let type_name = std::any::type_name::<D>();
        match self.id {
            Some(id) => write!(f, "dao-ref(key={}, type={})", id, type_name),
            None => write!(f, "dao-ref(type={})", type_name)
        }
    }
}

impl<D> DaoRef<D>
{
    pub fn new() -> DaoRef<D> {
        DaoRef {
            id: None,
            dio: DioWeak::Uninitialized,
            _marker: PhantomData,
        }
    }

    pub fn from_key(dio: &Arc<DioMut>, key: PrimaryKey) -> DaoRef<D> {
        DaoRef {
            id: Some(key),
            dio: DioWeak::from(&dio.dio),
            _marker: PhantomData,
        }
    }

    pub fn key(&self) -> Option<PrimaryKey> {
        self.id
    }

    pub fn set_key(&mut self, dio: &Arc<Dio>, val: PrimaryKey) {
        self.dio = DioWeak::from(dio);
        self.id = Some(val);
    }

    pub fn clear(&mut self) {
        self.id = None;
    }

    pub fn dio(&self) -> Option<Arc<Dio>> {
        match &self.dio {
            DioWeak::Uninitialized => None,
            DioWeak::Weak(a) => Weak::upgrade(a)
        }
    }

    /// Loads the data object (if it exists)
    pub async fn load(&self) -> Result<Option<Dao<D>>, LoadError>
    where D: DeserializeOwned,
    {
        let id = match self.id {
            Some(a) => a,
            None => {
                return Ok(None);
            }
        };

        let dio = match self.dio() {
            Some(a) => a,
            None => bail!(LoadErrorKind::WeakDio)
        };

        let ret = dio.load::<D>(&id).await?;
        Ok(Some(ret))
    }

    /// Stores the data within this reference
    pub fn store(&mut self, trans: &Arc<DioMut>, value: D) -> Result<DaoMut<D>, LoadError>
    where D: Serialize + DeserializeOwned,
    {
        let ret = trans.store::<D>(value)?;
        self.dio = DioWeak::from(trans);
        self.id = Some(ret.key().clone());
        Ok(ret)
    }

    /// Loads the data object or uses a default if none exists
    pub async fn unwrap_or(&mut self, default: D) -> Result<D, LoadError>
    where D: DeserializeOwned,
    {
        match self.id {
            Some(id) => {
                let dio = match self.dio() {
                    Some(a) => a,
                    None => bail!(LoadErrorKind::WeakDio)
                };
                Ok(dio.load::<D>(&id).await?.take())
            },
            None => {
                Ok(default)
            }
        }
    }

    /// Loads the data object or uses a default if none exists
    pub async fn unwrap_or_else<F: FnOnce() -> D>(&mut self, f: F) -> Result<D, LoadError>
    where D: Serialize + DeserializeOwned,
    {
        let dio = match self.dio() {
            Some(a) => a,
            None => bail!(LoadErrorKind::WeakDio)
        };

        match self.id {
            Some(id) => {
                Ok(dio.load::<D>(&id).await?.take())
            },
            None => {
                Ok(f())
            }
        }
    }

    /// Loads the data object or creates a new one (if it does not exist)
    pub async fn unwrap_or_default(&mut self)
    -> Result<D, LoadError>
    where D: Serialize + DeserializeOwned + Default
    {
        Ok(self.unwrap_or_else(|| {
            let ret: D = Default::default();
            ret
        }).await?)
    }

    pub async fn expect(&self, msg: &str) -> Dao<D>
    where D: DeserializeOwned,
    {
        match self.load().await {
            Ok(Some(a)) => a,
            Ok(None) => {
                panic!("{}", msg);
            }
            Err(err) => {
                panic!("{}: {:?}", msg, err);
            }
        }
    }

    pub async fn unwrap(&self) -> Dao<D>
    where D: DeserializeOwned,
    {
        self.load().await.ok().flatten().expect("called `DaoRef::unwrap()` that failed to load")
    }

    pub async fn take(&mut self) -> Result<Option<Dao<D>>, LoadError>
    where D: DeserializeOwned,
    {
        let key = self.id.take();
        self.id = None;
        
        let id = match key {
            Some(a) => a,
            None => {
                return Ok(None);
            }
        };

        let dio = match self.dio() {
            Some(a) => a,
            None => bail!(LoadErrorKind::WeakDio)
        };

        Ok(Some(dio.load::<D>(&id).await?))
    }

    pub async fn replace(&mut self, trans: &Arc<DioMut>, value: D) -> Result<Option<Dao<D>>, LoadError>
    where D: Serialize + DeserializeOwned,
    {
        let ret = trans.store::<D>(value)?;
        
        let key = self.id.replace(ret.key().clone());
        let id = match key {
            Some(a) => a,
            None => {
                return Ok(None);
            }
        };

        Ok(Some(trans.dio.load::<D>(&id).await?))
    }

    pub fn is_some(&self) -> bool {
        self.id.is_some()
    }

    pub fn is_none(&self) -> bool {
        !self.is_some()
    }
}