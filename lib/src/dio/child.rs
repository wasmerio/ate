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
use super::dio_mut::DioMutWeak;
use crate::dio::dao::*;
use crate::error::*;
use crate::header::*;
use super::vec::Iter;
use super::vec::IterMut;
use super::vec::DaoVecState as DaoChildState;

/// Rerepresents a reference to another data object with strong
/// type linting to make the model more solidified
///
#[derive(Serialize, Deserialize)]
pub struct DaoChild<D>
{
    pub(super) vec_id: u64,
    #[serde(skip)]
    pub(super) state: DaoChildState,
    #[serde(skip)]
    pub(super) dio: DioWeak,
    #[serde(skip)]
    pub(super) dio_mut: DioMutWeak,
    #[serde(skip)]
    pub(super) _marker: PhantomData<D>,
}

impl<D> std::fmt::Debug
for DaoChild<D>
where D: std::fmt::Debug
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let type_name = std::any::type_name::<D>();
        write!(f, "dao-child(type={})", type_name)
    }
}

impl<D> Default
for DaoChild<D>
{
    fn default() -> Self {
        DaoChild::new()
    }
}

impl<D> Clone
for DaoChild<D>
{
    fn clone(&self) -> Self
    {
        DaoChild {
            state: self.state.clone(),
            vec_id: self.vec_id,
            dio: self.dio.clone(),
            dio_mut: self.dio_mut.clone(),
            _marker: PhantomData,
        }
    }
}

impl<D> DaoChild<D>
{
    pub fn new() -> DaoChild<D> {
        DaoChild {
            state: DaoChildState::Unsaved,
            dio: DioWeak::Uninitialized,
            dio_mut: DioMutWeak::Uninitialized,
            vec_id: fastrand::u64(..),
            _marker: PhantomData,
        }
    }

    pub fn dio(&self) -> Option<Arc<Dio>> {
        match &self.dio {
            DioWeak::Uninitialized => None,
            DioWeak::Weak(a) => Weak::upgrade(a)
        }
    }

    pub fn dio_mut(&self) -> Option<Arc<DioMut>> {
        match &self.dio_mut {
            DioMutWeak::Uninitialized => None,
            DioMutWeak::Weak(a) => Weak::upgrade(a)
        }
    }

    async fn iter(&self) -> Result<Iter<D>, LoadError>
    where D: Serialize + DeserializeOwned
    {
        self.iter_ext(false, false).await
    }

    async fn iter_ext(&self, allow_missing_keys: bool, allow_serialization_error: bool) -> Result<Iter<D>, LoadError>
    where D: Serialize + DeserializeOwned
    {
        let children = match &self.state {
            DaoChildState::Unsaved => vec![],
            DaoChildState::Saved(parent_id) =>
            {
                if let Some(dio) = self.dio_mut() {
                    dio.children_ext(parent_id.clone(), self.vec_id, allow_missing_keys, allow_serialization_error).await?
                        .into_iter()
                        .rev()
                        .map(|a: DaoMut<D>| a.inner)
                        .collect::<Vec<_>>()
                } else {
                    let dio = match self.dio() {
                        Some(a) => a,
                        None => bail!(LoadErrorKind::WeakDio)
                    };                    
                    dio.children_ext(parent_id.clone(), self.vec_id, allow_missing_keys, allow_serialization_error).await?
                        .into_iter()
                        .rev()
                        .collect::<Vec<_>>()
                }
            },
        };

        Ok(
            Iter::new(
            children
            )
        )
    }

    async fn iter_mut(&mut self) -> Result<IterMut<D>, LoadError>
    where D: Serialize + DeserializeOwned
    {
        self.iter_mut_ext(false, false).await
    }

    async fn iter_mut_ext(&mut self, allow_missing_keys: bool, allow_serialization_error: bool) -> Result<IterMut<D>, LoadError>
    where D: Serialize + DeserializeOwned
    {
        let children = match &self.state {
            DaoChildState::Unsaved => vec![],
            DaoChildState::Saved(parent_id) =>
            {
                let dio = match self.dio_mut() {
                    Some(a) => a,
                    None => bail!(LoadErrorKind::WeakDio)
                };
                
                let mut ret = Vec::default();
                for child in dio.children_ext::<D>(parent_id.clone(), self.vec_id, allow_missing_keys, allow_serialization_error).await?
                    .into_iter()
                    .rev()
                {
                    ret.push(child)
                }
                ret
            },
        };

        Ok(
            IterMut::new(
            children                
            )
        )
    }
    
    pub async fn clear(&mut self) -> Result<(), LoadError>
    where D: Serialize + DeserializeOwned,
    {
        for child in self.iter_mut().await? {
            child.delete()?;
        }
        Ok(())
    }

    /// Loads the data object (if it exists)
    pub async fn load(&self) -> Result<Option<Dao<D>>, LoadError>
    where D: Serialize + DeserializeOwned,
    {
        Ok(self.iter().await?.next())
    }

    /// Loads the data object (if it exists)
    pub async fn load_mut(&mut self) -> Result<Option<DaoMut<D>>, LoadError>
    where D: Serialize + DeserializeOwned,
    {
        Ok(self.iter_mut().await?.next())
    }

    /// Stores the data within this reference
    pub async fn store(&mut self, data: D) -> Result<DaoMut<D>, LoadError>
    where D: Clone + Serialize + DeserializeOwned,
    {
        self.store_with_key(data, PrimaryKey::generate()).await
    }

    /// Stores the data within this reference
    pub async fn store_with_key(&mut self, data: D, key: PrimaryKey) -> Result<DaoMut<D>, LoadError>
    where D: Clone + Serialize + DeserializeOwned,
    {
        self.clear().await?;

        let dio = match self.dio_mut() {
            Some(a) => a,
            None => bail!(LoadErrorKind::WeakDio)
        };

        let parent_id = match &self.state {
            DaoChildState::Unsaved => { bail!(LoadErrorKind::SerializationError(SerializationErrorKind::SaveParentFirst)); },
            DaoChildState::Saved(a) => a.clone(),
        };

        let mut ret = dio.store_with_key(data, key)?;
        ret.attach_ext(parent_id, self.vec_id)?;
        Ok(ret)
    }

    /// Loads all the orphanes
    pub async fn orphans(&mut self) -> Result<IterMut<D>, LoadError>
    where D: Serialize + DeserializeOwned,
    {
        let mut iter = self.iter_mut().await?;
        let _top = iter.next();
        Ok(iter)
    }

    /// Loads the data object or uses a default if none exists
    pub async fn unwrap_or(&mut self, default: D) -> Result<D, LoadError>
    where D: Serialize + DeserializeOwned,
    {
        match self.load().await? {
            Some(a) => Ok(a.take()),
            None => {
                Ok(default)
            }
        }
    }

    /// Loads the data object or uses a default if none exists
    pub async fn unwrap_or_else<F: FnOnce() -> D>(&mut self, f: F) -> Result<D, LoadError>
    where D: Serialize + DeserializeOwned,
    {
        match self.load().await? {
            Some(a) => Ok(a.take()),
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
    where D: Serialize + DeserializeOwned,
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
    where D: Serialize + DeserializeOwned,
    {
        self.load().await.ok().flatten().expect("called `DaoRef::unwrap()` that failed to load")
    }

    pub async fn is_some(&self) -> Result<bool, LoadError>
    where D: Serialize + DeserializeOwned,
    {
        Ok(self.iter().await?.next().is_some())
    }

    pub async fn is_none(&self) -> Result<bool, LoadError>
    where D: Serialize + DeserializeOwned,
    {
        Ok(!self.is_some().await?)
    }
}

