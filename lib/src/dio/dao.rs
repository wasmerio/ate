#![allow(unused_imports)]
use log::{warn, debug};
use fxhash::FxHashSet;

use serde::{Serialize, de::DeserializeOwned};
use bytes::Bytes;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use parking_lot::{Mutex, MutexGuard};

use crate::crypto::{EncryptedPrivateKey, PrivateKey};
use crate::{crypto::EncryptKey, session::{Session, SessionProperty}};

use crate::header::*;
use crate::event::*;
use crate::meta::*;
use crate::error::*;
use crate::crypto::Hash;
use crate::dio::DioState;
use crate::dio::Dio;
use crate::spec::*;
use crate::index::*;

pub use super::vec::DaoVec;

#[derive(Debug, Clone)]
pub(super) struct Row<D>
where Self: Send + Sync,
      D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    pub(super) key: PrimaryKey,
    pub(super) created: u64,
    pub(super) updated: u64,
    pub(super) format: MessageFormat,
    pub(super) tree: Option<MetaTree>,
    pub(super) data: D,
    pub(super) auth: MetaAuthorization,
    pub(super) collections: FxHashSet<MetaCollection>,
}

impl<D> Row<D>
where Self: Send + Sync,
      D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    pub(crate) fn from_event(evt: &EventData, created: u64, updated: u64) -> Result<Row<D>, SerializationError> {
        let key = match evt.meta.get_data_key() {
            Some(key) => key,
            None => { return Result::Err(SerializationError::NoPrimarykey) }
        };
        let mut collections = FxHashSet::default();
        for a in evt.meta.get_collections() {
            collections.insert(a);
        }
        match &evt.data_bytes {
            Some(data) => {
                Ok(
                    Row {
                        key,
                        format: evt.format,
                        tree: match evt.meta.get_tree() { Some(a) => Some(a.clone()), None => None },
                        data: evt.format.data.deserialize(&data)?,
                        auth: match evt.meta.get_authorization() {
                            Some(a) => a.clone(),
                            None => MetaAuthorization::default(),
                        },
                        collections,
                        created,
                        updated,
                    }
                )
            }
            None => return Result::Err(SerializationError::NoData),
        }
    }

    pub(crate) fn from_row_data(row: &RowData) -> Result<Row<D>, SerializationError> {
        Ok(
            Row {
                key: row.key,
                format: row.format,
                tree: row.tree.clone(),
                data: row.format.data.deserialize(&row.data)?,
                auth: row.auth.clone(),
                collections: row.collections.clone(),
                created: row.created,
                updated: row.updated,
            }
        )
    }

    pub(crate) fn as_row_data(&self) -> std::result::Result<RowData, SerializationError> {
        let data = Bytes::from(self.format.data.serialize(&self.data)?);
            
        let data_hash = Hash::from_bytes(&data[..]);
        Ok
        (
            RowData {
                key: self.key.clone(),
                format: self.format,
                tree: self.tree.clone(),
                data_hash,
                data,
                auth: self.auth.clone(),
                collections: self.collections.clone(),
                created: self.created,
                updated: self.updated,
            }
        )
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RowData
where Self: Send + Sync
{
    pub key: PrimaryKey,
    pub format: MessageFormat,
    pub tree: Option<MetaTree>,
    pub data_hash: Hash,
    pub data: Bytes,
    pub auth: MetaAuthorization,
    pub collections: FxHashSet<MetaCollection>,
    pub created: u64,
    pub updated: u64,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DaoLock
{
    /// The DAO has no lock on it
    Unlocked,
    /// The DAO has been manually locked forcing serial access
    Locked,
    /// The dao is being processed thus holds a lock and should be deleted
    /// when it goes out of scope
    LockedThenDelete,
}

#[derive(Debug)]
pub struct DaoState
where Self: Send + Sync,
{
    pub(super) lock: DaoLock,
    pub(super) dirty: bool,
}

/// Represents a data object that will be represented as one or
/// more events on the redo-log and validated in the chain-of-trust.
/// 
/// Reading this object using none-mutable behavior will incur no IO
/// on the redo-log however if you edit the object you must commit it
/// to the `Dio` before it goes out of scope or the data will be lost
/// (in Debug mode this will even trigger an assert).
///
/// Metadata about the data object can also be accessed via this object
/// which allows you to change the read/write access rights, etc.
///
/// If you change your mind on commiting the data to the redo-log then
/// you can call the `cancel` function instead.
#[derive(Debug)]
pub struct Dao<D>
where Self: Send + Sync,
      D: Serialize + DeserializeOwned + Clone + Sync + Send,
{
    pub(super) state: DaoState,
    pub(super) row: Row<D>,
}

impl<D> Dao<D>
where Self: Send + Sync,
      D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    pub(super) fn new<>(row: Row<D>) -> Dao<D> {
        Dao {
            state: DaoState {
                lock: DaoLock::Unlocked,
                dirty: false,
            },
            row: row,
        }
    }

    pub fn make(key: PrimaryKey, format: MessageFormat, data: D) -> Dao<D> {
        Dao {
            state: DaoState {
                lock: DaoLock::Unlocked,
                dirty: true,
            },
            row: Row {
                key,
                created: 0,
                updated: 0,
                tree: None,
                data,
                format: format,
                auth: MetaAuthorization {
                    read: ReadOption::Unspecified,
                    write: WriteOption::Unspecified,
                },
                collections: FxHashSet::default(),
            },
        }
    }

    #[allow(dead_code)]
    pub(super) fn attach_vec<C>(&mut self, vec: &MetaCollection)
    where C: Serialize + DeserializeOwned + Clone,
    {
        if self.row.collections.contains(vec) {
            return;
        }

        self.state.dirty = true;
        self.row.collections.insert(vec.clone());
    }

    #[allow(dead_code)]
    pub fn key(&self) -> &PrimaryKey {
        &self.row.key
    }

    #[allow(dead_code)]
    pub fn detach(&mut self) {
        self.state.dirty = true;
        self.row.tree = None;
    }

    #[allow(dead_code)]
    pub fn attach(&mut self, parent_id: &PrimaryKey, vec: &DaoVec<D>) {
        self.state.dirty = true;
        self.row.tree = Some(
            MetaTree {
                vec: MetaCollection {
                    parent_id: parent_id.clone(),
                    collection_id: vec.vec_id.clone(),
                },
                inherit_read: true,
                inherit_write: true,
            }
        );
    }

    #[allow(dead_code)]
    pub fn auth(&self) -> &MetaAuthorization {
        &self.row.auth
    }

    #[allow(dead_code)]
    pub fn auth_mut(&mut self) -> &mut MetaAuthorization {
        self.state.dirty = true;
        &mut self.row.auth
    }

    #[allow(dead_code)]
    pub fn delete<'a>(self, dio: &mut Dio<'a>) -> std::result::Result<(), SerializationError> {
        let state = &mut dio.state;
        delete_internal(&self, state)
    }

    #[allow(dead_code)]
    pub async fn try_lock<'a>(&mut self, dio: &mut Dio<'a>) -> Result<bool, LockError> {
        self.state.dirty = true;

        match self.state.lock {
            DaoLock::Locked | DaoLock::LockedThenDelete => {},
            DaoLock::Unlocked =>
            {
                // Attempt the lock
                if dio.multi.pipe.try_lock(self.key().clone()).await? == false {
                    return Ok(false)
                }

                // The object is now locked
                self.state.lock = DaoLock::Locked;
            }
        };
        Ok(true)
    }

    #[allow(dead_code)]
    pub async fn try_lock_then_delete<'a>(&mut self, dio: &mut Dio<'a>) -> Result<bool, LockError> {
        if self.try_lock(dio).await? == false {
            return Ok(false);
        }
        self.state.lock = DaoLock::LockedThenDelete;
        Ok(true)
    }

    #[allow(dead_code)]
    pub async fn unlock<'a>(&mut self, dio: &mut Dio<'a>) -> Result<bool, LockError> {
        match self.state.lock {
            DaoLock::Unlocked | DaoLock::LockedThenDelete => {
                return Ok(false);
            },
            DaoLock::Locked => {
                dio.multi.pipe.unlock(self.key().clone()).await?;
                self.state.lock = DaoLock::Unlocked;
            }
        };

        Ok(true)
    }

    #[allow(dead_code)]
    pub fn is_locked(&self) -> bool {
        match self.state.lock {
            DaoLock::Locked | DaoLock::LockedThenDelete => true,
            DaoLock::Unlocked => false
        }
    }

    #[allow(dead_code)]
    pub fn is_dirty(&self) -> bool {
        self.state.dirty
    }

    #[allow(dead_code)]
    pub fn when_created(&self) -> u64 {
        self.row.created
    }

    #[allow(dead_code)]
    pub fn when_updated(&self) -> u64 {
        self.row.updated
    }

    pub fn cancel(&mut self) {
        self.state.dirty = false;
    }
    
    pub fn commit<'a>(&mut self, dio: &mut Dio<'a>) -> std::result::Result<(), SerializationError>
    {
        if self.state.dirty == true {
            self.state.dirty = false;

            let state = &mut dio.state;

            // The local DIO lock gets released first
            state.unlock(&self.row.key);

            // Next any pessimistic locks on the local chain
            match self.state.lock {
                DaoLock::Locked => {
                    state.pipe_unlock.insert(self.row.key.clone());
                },
                DaoLock::LockedThenDelete => {
                    state.pipe_unlock.insert(self.row.key.clone());
                    delete_internal(self, state)?;
                    return Ok(())
                },
                _ => {}
            }

            let row_data = self.row.as_row_data()?;
            let row_tree = match &self.row.tree {
                Some(a) => Some(a),
                None => None,
            };
            state.dirty(&self.row.key, row_tree, row_data);
        }
        Ok(())
    }
    
    pub fn take(self) -> D {
        self.row.data
    }
}

pub(crate) fn delete_internal<D>(dao: &Dao<D>, state: &mut DioState)
    -> std::result::Result<(), SerializationError>
where D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    if state.lock(&dao.row.key) == false {
        eprintln!("Detected concurrent write while deleting a data object ({:?}) - the delete operation will override everything else", dao.row.key);
    }
    let key = dao.key().clone();
    state.cache_store_primary.remove(&key);
    if let Some(tree) = &dao.row.tree {
        if let Some(y) = state.cache_store_secondary.get_vec_mut(&tree.vec) {
            y.retain(|x| *x == key);
        }
    }
    state.cache_load.remove(&key);

    let row_data = dao.row.as_row_data()?;
    state.deleted.insert(key, row_data);
    Ok(())
}

impl<D> Deref for Dao<D>
where D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    type Target = D;

    fn deref(&self) -> &Self::Target {
        &self.row.data
    }
}

impl<D> DerefMut for Dao<D>
where D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.state.dirty = true;
        &mut self.row.data
    }
}

impl Drop for DaoState
{
    fn drop(&mut self)
    {
        // If the DAO is dirty and was not committed then assert an error
        debug_assert!(self.dirty == false, "dao-is-dirty - the DAO is dirty due to mutations thus you must call .commit() or .cancel()");
    }
}