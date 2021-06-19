#![allow(unused_imports)]
use log::{warn, debug};
use fxhash::FxHashSet;

use serde::{Serialize, de::DeserializeOwned};
use bytes::Bytes;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use parking_lot::{Mutex, MutexGuard};

use crate::crypto::{EncryptedPrivateKey, PrivateSignKey};
use crate::{crypto::EncryptKey, session::{AteSession, AteSessionProperty}};

use crate::header::*;
use crate::event::*;
use crate::meta::*;
use crate::error::*;
use crate::crypto::AteHash;
use crate::dio::*;
use crate::spec::*;
use crate::index::*;

pub use super::vec::DaoVec;

#[derive(Debug, Clone)]
pub(super) struct Row<D>
where Self: Send + Sync,
      D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    pub(super) key: PrimaryKey,
    pub(super) type_name: &'static str,
    pub(super) created: u64,
    pub(super) updated: u64,
    pub(super) format: MessageFormat,
    pub(super) parent: Option<MetaParent>,
    pub(super) data: D,
    pub(super) auth: MetaAuthorization,
    pub(super) collections: FxHashSet<MetaCollection>,
    pub(super) extra_meta: Vec<CoreMetadata>,
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
                let auth = match evt.meta.get_authorization() {
                    Some(a) => a.clone(),
                    None => MetaAuthorization::default(),
                };
                let parent = match evt.meta.get_parent() { Some(a) => Some(a.clone()), None => None };
                Ok(
                    Row {
                        key,
                        type_name: std::any::type_name::<D>(),
                        format: evt.format,
                        parent,
                        data: evt.format.data.deserialize(&data)?,
                        auth,
                        collections,
                        created,
                        updated,
                        extra_meta: Vec::new(),
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
                type_name: row.type_name,
                format: row.format,
                parent: row.parent.clone(),
                data: row.format.data.deserialize(&row.data)?,
                auth: row.auth.clone(),
                collections: row.collections.clone(),
                created: row.created,
                updated: row.updated,
                extra_meta: row.extra_meta.clone(),
            }
        )
    }

    pub(crate) fn as_row_data(&self) -> std::result::Result<RowData, SerializationError> {
        let data = Bytes::from(self.format.data.serialize(&self.data)?);
            
        let data_hash = AteHash::from_bytes(&data[..]);
        Ok
        (
            RowData {
                key: self.key.clone(),
                type_name: self.type_name,
                format: self.format,
                parent: self.parent.clone(),
                data_hash,
                data,
                auth: self.auth.clone(),
                collections: self.collections.clone(),
                created: self.created,
                updated: self.updated,
                extra_meta: self.extra_meta.clone(),
            }
        )
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RowData
where Self: Send + Sync
{
    pub key: PrimaryKey,
    pub type_name: &'static str,
    pub format: MessageFormat,
    pub parent: Option<MetaParent>,
    pub data_hash: AteHash,
    pub data: Bytes,
    pub auth: MetaAuthorization,
    pub collections: FxHashSet<MetaCollection>,
    pub created: u64,
    pub updated: u64,
    pub extra_meta: Vec<CoreMetadata>,
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
    pub(super) auto_cancel: bool,
}

pub trait DaoObjEthereal
{
    fn auth(&self) -> &MetaAuthorization;

    fn auth_mut(&mut self) -> &mut MetaAuthorization;

    fn add_extra_metadata(&mut self, meta: CoreMetadata);

    fn is_locked(&self) -> bool;

    fn is_dirty(&self) -> bool;

    fn cancel(&mut self);

    fn auto_cancel(&mut self);
}

pub trait DaoObjReal
{
    fn key(&self) -> &PrimaryKey;

    fn delete<'a>(self, dio: &mut Dio<'a>) -> std::result::Result<(), SerializationError>;

    fn when_created(&self) -> u64;

    fn when_updated(&self) -> u64;
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
///
/// The ethereal version represents all operations that can be performed
/// before the obejct is actually saved
#[derive(Debug)]
pub struct DaoEthereal<D>
where Self: Send + Sync,
      D: Serialize + DeserializeOwned + Clone + Sync + Send,
{
    pub(super) state: DaoState,
    pub(super) row: Row<D>,
}

impl<D> DaoEthereal<D>
where Self: Send + Sync,
      D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    pub(super) fn new<>(row: Row<D>) -> DaoEthereal<D> {
        DaoEthereal {
            state: DaoState {
                lock: DaoLock::Unlocked,
                dirty: false,
                auto_cancel: false,
            },
            row: row,
        }
    }

    pub fn make(key: PrimaryKey, format: MessageFormat, data: D) -> DaoEthereal<D> {
        DaoEthereal {
            state: DaoState {
                lock: DaoLock::Unlocked,
                dirty: true,
                auto_cancel: false,
            },
            row: Row {
                key,
                type_name: std::any::type_name::<D>(),
                created: 0,
                updated: 0,
                parent: None,
                data,
                format,
                auth: MetaAuthorization {
                    read: ReadOption::Inherit,
                    write: WriteOption::Inherit,
                },
                collections: FxHashSet::default(),
                extra_meta: Vec::new(),
            },
        }
    }

    pub fn detach(&mut self) {
        self.state.dirty = true;
        self.row.parent = None;
    }

    pub fn attach(&mut self, parent: &dyn DaoObjReal, vec: DaoVec<D>) {
        self.state.dirty = true;
        self.row.parent = Some(
            MetaParent {
                vec: MetaCollection {
                    parent_id: parent.key().clone(),
                    collection_id: vec.vec_id,
                },
            }
        );
    }

    pub fn attach_orphaned(&mut self, parent: &PrimaryKey) {
        self.attach_orphaned_ext(parent.clone(), 0u64)
    }

    pub fn attach_orphaned_ext(&mut self, parent: PrimaryKey, collection_id: u64) {
        self.state.dirty = true;
        self.row.parent = Some(
            MetaParent {
                vec: MetaCollection {
                    parent_id: parent,
                    collection_id,
                },
            }
        );
    }
    
    pub fn take(self) -> D {
        self.row.data
    }

    pub async fn try_lock<'a>(&mut self, dio: &mut Dio<'a>) -> Result<bool, LockError> {
        self.state.dirty = true;

        match self.state.lock {
            DaoLock::Locked | DaoLock::LockedThenDelete => {},
            DaoLock::Unlocked =>
            {
                // Attempt the lock
                if dio.multi.pipe.try_lock(self.row.key.clone()).await? == false {
                    return Ok(false)
                }

                // The object is now locked
                self.state.lock = DaoLock::Locked;
            }
        };
        Ok(true)
    }

    pub async fn unlock<'a>(&mut self, dio: &mut Dio<'a>) -> Result<bool, LockError> {
        match self.state.lock {
            DaoLock::Unlocked | DaoLock::LockedThenDelete => {
                return Ok(false);
            },
            DaoLock::Locked => {
                dio.multi.pipe.unlock(self.row.key.clone()).await?;
                self.state.lock = DaoLock::Unlocked;
            }
        };

        Ok(true)
    }
    
    pub fn commit<'a>(self, dio: &mut Dio<'a>) -> std::result::Result<Dao<D>, SerializationError>
    {
        let mut ret = Dao::new(self);
        ret.commit(dio)?;
        Ok(ret)
    }
}

impl<D> DaoObjEthereal
for DaoEthereal<D>
where Self: Send + Sync,
      D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    fn auth(&self) -> &MetaAuthorization {
        &self.row.auth
    }

    fn auth_mut(&mut self) -> &mut MetaAuthorization {
        self.state.dirty = true;
        &mut self.row.auth
    }

    fn auto_cancel(&mut self) {
        self.state.auto_cancel = true;
    }

    fn add_extra_metadata(&mut self, meta: CoreMetadata) {
        self.state.dirty = true;
        self.row.extra_meta.push(meta);
    }

    fn is_locked(&self) -> bool {
        match self.state.lock {
            DaoLock::Locked | DaoLock::LockedThenDelete => true,
            DaoLock::Unlocked => false
        }
    }

    fn is_dirty(&self) -> bool {
        self.state.dirty
    }

    fn cancel(&mut self) {
        self.state.cancel();
    }
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
///
/// The real version represents all operations that can be performed
/// before the obejct is actually saved and all those after
#[derive(Debug)]
pub struct Dao<D>
where Self: Send + Sync,
      D: Serialize + DeserializeOwned + Clone + Sync + Send,
{
    pub(super) ethereal: DaoEthereal<D>,
}

impl<D> Dao<D>
where Self: Send + Sync,
      D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    pub(super) fn new<>(ethereal: DaoEthereal<D>) -> Dao<D> {
        Dao {
            ethereal
        }
    }

    pub fn make(key: PrimaryKey, format: MessageFormat, data: D) -> DaoEthereal<D> {
        DaoEthereal::make(key, format, data)
    }

    pub fn detach(&mut self) {
        self.ethereal.detach()
    }

    pub fn attach(&mut self, parent: &dyn DaoObjReal, vec: DaoVec<D>) {
        self.ethereal.attach(parent, vec)
    }

    pub fn attach_orphaned(&mut self, parent: &PrimaryKey) {
        self.ethereal.attach_orphaned(parent)
    }
    
    pub fn take(self) -> D {
        self.ethereal.take()
    }

    pub async fn try_lock<'a>(&mut self, dio: &mut Dio<'a>) -> Result<bool, LockError> {
        self.ethereal.try_lock(dio).await
    }

    pub async fn try_lock_then_delete<'a>(&mut self, dio: &mut Dio<'a>) -> Result<bool, LockError> {
        if self.try_lock(dio).await? == false {
            return Ok(false);
        }
        self.ethereal.state.lock = DaoLock::LockedThenDelete;
        Ok(true)
    }

    pub async fn unlock<'a>(&mut self, dio: &mut Dio<'a>) -> Result<bool, LockError> {
        self.ethereal.unlock(dio).await
    }

    pub fn commit<'a>(&mut self, dio: &mut Dio<'a>) -> std::result::Result<(), SerializationError>
    {
        let s = &mut self.ethereal;
        if s.state.dirty == true {
            s.state.dirty = false;

            let state = &mut dio.state;

            // The local DIO lock gets released first
            state.unlock(&s.row.key);

            // Next any pessimistic locks on the local chain
            match s.state.lock {
                DaoLock::Locked => {
                    state.pipe_unlock.insert(s.row.key.clone());
                },
                DaoLock::LockedThenDelete => {
                    state.pipe_unlock.insert(s.row.key.clone());
                    delete_internal(self, state)?;
                    return Ok(())
                },
                _ => {}
            }

            let row_data = s.row.as_row_data()?;
            let row_parent = match &s.row.parent {
                Some(a) => Some(a),
                None => None,
            };
            state.dirty(&s.row.key, row_parent, row_data);
        }
        Ok(())
    }
}

impl<D> DaoObjReal
for Dao<D>
where Self: Send + Sync,
      D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    fn key(&self) -> &PrimaryKey {
        &self.ethereal.row.key
    }

    fn delete<'a>(self, dio: &mut Dio<'a>) -> std::result::Result<(), SerializationError> {
        let state = &mut dio.state;
        delete_internal(&self, state)
    }

    fn when_created(&self) -> u64 {
        self.ethereal.row.created
    }

    fn when_updated(&self) -> u64 {
        self.ethereal.row.updated
    }
}

impl<D> DaoObjEthereal
for Dao<D>
where Self: Send + Sync,
      D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    fn auth(&self) -> &MetaAuthorization {
        self.ethereal.auth()
    }

    fn auth_mut(&mut self) -> &mut MetaAuthorization {
        self.ethereal.auth_mut()
    }

    fn add_extra_metadata(&mut self, meta: CoreMetadata) {
        self.ethereal.add_extra_metadata(meta)
    }

    fn is_locked(&self) -> bool {
        self.ethereal.is_locked()
    }

    fn is_dirty(&self) -> bool {
        self.ethereal.is_dirty()
    }

    fn cancel(&mut self) {
        self.ethereal.cancel()
    }

    fn auto_cancel(&mut self) {
        self.ethereal.auto_cancel()
    }
}

pub(crate) fn delete_internal<D>(dao: &Dao<D>, state: &mut DioState)
    -> std::result::Result<(), SerializationError>
where D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    let key = dao.key().clone();
    state.add_deleted(key, dao.ethereal.row.parent.clone());
    Ok(())
}

impl<D> std::ops::Deref for Dao<D>
where D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    type Target = D;

    fn deref(&self) -> &Self::Target {
        self.ethereal.deref()
    }
}

impl<D> std::ops::DerefMut for Dao<D>
where D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ethereal.deref_mut()
    }
}

impl<D> std::ops::Deref for DaoEthereal<D>
where D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    type Target = D;

    fn deref(&self) -> &Self::Target {
        &self.row.data
    }
}

impl<D> std::ops::DerefMut for DaoEthereal<D>
where D: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.state.dirty = true;
        &mut self.row.data
    }
}

impl DaoState
{
    fn cancel(&mut self) {
        self.dirty = false;
    }
}

impl Drop for DaoState
{
    fn drop(&mut self)
    {
        // Check if auto-cancel is enabled
        if self.dirty & self.auto_cancel {
            debug!("Data objects have been discarded due to auto-cancel and uncommitted changes");
            self.cancel();
        }

        // If the DAO is dirty and was not committed then assert an error
        debug_assert!(self.dirty == false, "dao-is-dirty - the DAO is dirty due to mutations thus you must call .commit() or .cancel()");
    }
}