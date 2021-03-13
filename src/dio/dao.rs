use log::{warn};
use fxhash::FxHashSet;

use serde::{Serialize, de::DeserializeOwned};
use bytes::Bytes;
use std::cell::{RefCell, RefMut};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

#[allow(unused_imports)]
use crate::crypto::{EncryptedPrivateKey, PrivateKey};
#[allow(unused_imports)]
use crate::{crypto::EncryptKey, session::{Session, SessionProperty}};

use crate::header::*;
use crate::event::*;
use crate::meta::*;
use crate::error::*;
use crate::crypto::Hash;
use crate::dio::DioState;
use crate::dio::Dio;

pub use super::vec::DaoVec;

#[derive(Debug, Clone)]
pub(super) struct Row<D>
where D: Serialize + DeserializeOwned + Clone,
{
    pub(super) key: PrimaryKey,
    pub(super) tree: Option<MetaTree>,
    pub(super) data: D,
    pub(super) auth: MetaAuthorization,
    pub(super) collections: FxHashSet<MetaCollection>,
}

impl<D> Row<D>
where D: Serialize + DeserializeOwned + Clone,
{
    #[allow(dead_code)]
    pub(super) fn new(
        key: PrimaryKey,
        data: D,
        auth: MetaAuthorization,
        tree: Option<MetaTree>,
        collections: FxHashSet<MetaCollection>,
    ) -> Row<D>
    {
        Row {
            key,
            tree,
            data,
            auth,
            collections,
        }
    }

    pub fn from_event(evt: &EventExt) -> Result<Row<D>, SerializationError> {
        let key = match evt.raw.meta.get_data_key() {
            Some(key) => key,
            None => { return Result::Err(SerializationError::NoPrimarykey) }
        };
        let mut collections = FxHashSet::default();
        for a in evt.raw.meta.get_collections() {
            collections.insert(a);
        }
        match &evt.raw.data {
            Some(data) => {
                Ok(
                    Row {
                        key,
                        tree: match evt.raw.meta.get_tree() { Some(a) => Some(a.clone()), None => None },
                        data: serde_json::from_slice(&data)?,
                        auth: match evt.raw.meta.get_authorization() {
                            Some(a) => a.clone(),
                            None => MetaAuthorization::default(),
                        },
                        collections,
                    }
                )
            }
            None => return Result::Err(SerializationError::NoData),
        }
    }

    pub fn from_row_data(row: &RowData) -> Result<Row<D>, SerializationError> {
        Ok(
            Row {
                key: row.key,
                tree: row.tree.clone(),
                data: serde_json::from_slice(&row.data)?,
                auth: row.auth.clone(),
                collections: row.collections.clone(),
            }
        )
    }

    pub fn as_row_data(&self) -> std::result::Result<RowData, SerializationError> {
        let data = Bytes::from(serde_json::to_vec(&self.data)?);
            
        let data_hash = Hash::from_bytes(&data[..]);
        Ok
        (
            RowData {
                key: self.key.clone(),
                tree: self.tree.clone(),
                data_hash,
                data,
                auth: self.auth.clone(),
                collections: self.collections.clone(),
            }
        )
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RowData
{
    pub key: PrimaryKey,
    pub tree: Option<MetaTree>,
    pub data_hash: Hash,
    pub data: Bytes,
    pub auth: MetaAuthorization,
    pub collections: FxHashSet<MetaCollection>,
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
pub struct Dao<D>
where D: Serialize + DeserializeOwned + Clone,
{
    lock: DaoLock,
    dirty: bool,
    pub(super) row: Row<D>,
    state: Rc<RefCell<DioState>>,
}

impl<D> Dao<D>
where D: Serialize + DeserializeOwned + Clone,
{
    pub(super) fn new<>(row: Row<D>, state: &Rc<RefCell<DioState>>) -> Dao<D> {
        Dao {
            lock: DaoLock::Unlocked,
            dirty: false,
            row: row,
            state: Rc::clone(state),
        }
    }

    pub(super) fn fork(&mut self) -> bool {
        if self.dirty == false {
            let mut state = self.state.borrow_mut();
            if state.lock(&self.row.key) == false {
                eprintln!("Detected concurrent writes on data object ({:?}) - the last one in scope will override the all other changes made", self.row.key);
            }
            self.dirty = true;
        }
        true
    }

    #[allow(dead_code)]
    pub(super) fn attach_vec<C>(&mut self, vec: &MetaCollection)
    where C: Serialize + DeserializeOwned + Clone,
    {
        if self.row.collections.contains(vec) {
            return;
        }

        self.fork();
        self.row.collections.insert(vec.clone());
    }

    #[allow(dead_code)]
    pub fn key(&self) -> &PrimaryKey {
        &self.row.key
    }

    #[allow(dead_code)]
    pub fn detach(&mut self) {
        self.fork();
        self.row.tree = None;
    }

    #[allow(dead_code)]
    pub fn auth(&self) -> &MetaAuthorization {
        &self.row.auth
    }

    #[allow(dead_code)]
    pub fn auth_mut(&mut self) -> &mut MetaAuthorization {
        self.fork();
        &mut self.row.auth
    }

    #[allow(dead_code)]
    pub(crate) fn delete(self) -> std::result::Result<(), SerializationError> {
        let mut state = self.state.borrow_mut();
        self.delete_internal(&mut state)
    }

    #[allow(dead_code)]
    pub(crate) fn delete_internal(&self, state: &mut RefMut<DioState>) -> std::result::Result<(), SerializationError> {
        if state.lock(&self.row.key) == false {
            eprintln!("Detected concurrent write while deleting a data object ({:?}) - the delete operation will override everything else", self.row.key);
        }
        let key = self.key().clone();
        state.cache_store_primary.remove(&key);
        if let Some(tree) = &self.row.tree {
            if let Some(y) = state.cache_store_secondary.get_vec_mut(&tree.vec) {
                y.retain(|x| *x == key);
            }
        }
        state.cache_load.remove(&key);

        let row_data = self.row.as_row_data()?;
        state.deleted.insert(key, Rc::new(row_data));
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn try_lock<'a>(&mut self, dio: &mut Dio<'a>) -> Result<bool, LockError> {
        self.fork();

        match self.lock {
            DaoLock::Locked | DaoLock::LockedThenDelete => {},
            DaoLock::Unlocked =>
            {
                // Attempt the lock
                if dio.multi.pipe.try_lock(self.key().clone()).await? == false {
                    return Ok(false)
                }

                // The object is now locked
                self.lock = DaoLock::Locked;
            }
        };
        Ok(true)
    }

    #[allow(dead_code)]
    pub async fn try_lock_then_delete<'a>(&mut self, dio: &mut Dio<'a>) -> Result<bool, LockError> {
        if self.try_lock(dio).await? == false {
            return Ok(false);
        }
        self.lock = DaoLock::LockedThenDelete;
        Ok(true)
    }

    #[allow(dead_code)]
    pub async fn unlock<'a>(&mut self, dio: &mut Dio<'a>) -> Result<bool, LockError> {
        match self.lock {
            DaoLock::Unlocked | DaoLock::LockedThenDelete => {
                return Ok(false);
            },
            DaoLock::Locked => {
                dio.multi.pipe.unlock(self.key().clone()).await?;
                self.lock = DaoLock::Unlocked;
            }
        };

        Ok(true)
    }

    #[allow(dead_code)]
    pub fn is_locked(&self) -> bool {
        match self.lock {
            DaoLock::Locked | DaoLock::LockedThenDelete => true,
            DaoLock::Unlocked => false
        }
    }
}

impl<D> Deref for Dao<D>
where D: Serialize + DeserializeOwned + Clone,
{
    type Target = D;

    fn deref(&self) -> &Self::Target {
        &self.row.data
    }
}

impl<D> DerefMut for Dao<D>
where D: Serialize + DeserializeOwned + Clone,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.fork();
        &mut self.row.data
    }
}

impl<D> Drop for Dao<D>
where D: Serialize + DeserializeOwned + Clone,
{
    fn drop(&mut self)
    {
        // Now attempt to flush it
        if let Err(err) = self.flush() {
            debug_assert!(false, "dao-flush-error {}", err.to_string());
            warn!("dao-flush-error {}", err.to_string());
        }
    }
}

impl<D> Dao<D>
where D: Serialize + DeserializeOwned + Clone,
{
    pub fn flush(&mut self) -> std::result::Result<(), SerializationError>
    {
        if self.dirty == true
        {            
            let mut state = self.state.borrow_mut();

            // The local DIO lock gets released first
            state.unlock(&self.row.key);

            // Next any pessimistic locks on the local chain
            match self.lock {
                DaoLock::Locked => {
                    state.pipe_unlock.insert(self.row.key.clone());
                },
                DaoLock::LockedThenDelete => {
                    state.pipe_unlock.insert(self.row.key.clone());
                    self.delete_internal(&mut state)?;
                },
                _ => {}
            }
            
            // Next we ship any data
            self.dirty = false;

            let row_data = self.row.as_row_data()?;
            let row_tree = match &self.row.tree {
                Some(a) => Some(a),
                None => None,
            };
            state.dirty(&self.row.key, row_tree, row_data);
        }
        Ok(())
    }
}