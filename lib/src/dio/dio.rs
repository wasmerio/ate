#![allow(unused_imports)]
use log::{info, error, debug};
use crate::prelude::*;
use fxhash::FxHashMap;
use fxhash::FxHashSet;
use multimap::MultiMap;
use serde::{Deserialize};
use serde::{Serialize, de::DeserializeOwned};
use std::{fmt::Debug, sync::Arc};
use parking_lot::Mutex;
use std::ops::Deref;
use tokio::sync::mpsc;
use std::sync::mpsc as smpsc;

use super::dao::*;
use crate::meta::*;
use crate::event::*;
use crate::tree::*;
use crate::index::*;
use crate::transaction::*;
use crate::comms::*;
use crate::spec::*;
use crate::error::*;
use crate::lint::*;

use crate::crypto::{EncryptedPrivateKey, PrivateSignKey};
use crate::{crypto::EncryptKey, session::{Session, SessionProperty}};

#[derive(Debug)]
pub(crate) struct DioState
where Self: Send + Sync
{
    pub(super) store: Vec<Arc<RowData>>,
    pub(super) cache_store_primary: FxHashMap<PrimaryKey, Arc<RowData>>,
    pub(super) cache_store_secondary: MultiMap<MetaCollection, PrimaryKey>,
    pub(super) cache_load: FxHashMap<PrimaryKey, (Arc<EventData>, EventLeaf)>,
    pub(super) locked: FxHashSet<PrimaryKey>,
    pub(super) deleted: FxHashSet<PrimaryKey>,
    pub(super) pipe_unlock: FxHashSet<PrimaryKey>,
}

impl DioState
{
    pub(super) fn dirty(&mut self, key: &PrimaryKey, parent: Option<&MetaParent>, row: RowData) {
        let row = Arc::new(row);
        self.store.push(row.clone());
        self.cache_store_primary.insert(key.clone(), row);
        if let Some(parent) = parent {
            self.cache_store_secondary.insert(parent.vec.clone(), key.clone());
        }
        self.cache_load.remove(key);
    }

    pub(super) fn lock(&mut self, key: &PrimaryKey) -> bool {
        self.locked.insert(key.clone())
    }

    pub(super) fn unlock(&mut self, key: &PrimaryKey) -> bool {
        self.locked.remove(key)
    }

    pub(super) fn is_locked(&self, key: &PrimaryKey) -> bool {
        self.locked.contains(key)
    }

    pub(super) fn add_deleted(&mut self, key: PrimaryKey, parent: Option<MetaParent>)
    {
        if self.lock(&key) == false {
            eprintln!("Detected concurrent write while deleting a data object ({:?}) - the delete operation will override everything else", key);
        }

        self.cache_store_primary.remove(&key);
        if let Some(tree) = parent {
            if let Some(y) = self.cache_store_secondary.get_vec_mut(&tree.vec) {
                y.retain(|x| *x == key);
            }
        }
        self.cache_load.remove(&key);
        self.deleted.insert(key);
    }
}

impl DioState
{
    #[allow(dead_code)]
    fn new() -> DioState {
        DioState {
            store: Vec::new(),
            cache_store_primary: FxHashMap::default(),
            cache_store_secondary: MultiMap::new(),
            cache_load: FxHashMap::default(),
            locked: FxHashSet::default(),
            deleted: FxHashSet::default(),
            pipe_unlock: FxHashSet::default(),
        }
    }
}

/// Represents a series of mutations that the user is making on a particular chain-of-trust
/// with a specific set of facts attached to a session. All changes are stored in memory
/// until the commit function is invoked which will feed them into the chain.
///
/// If you decide to abort the transaction then call the `cancel` function before it goes
/// out of scope however if you mutate data and do not call `commit` then the data will be
/// lost (or an assert will be triggerd when in Debug mode).
///
/// These objects are multi-thread safe and allow for very high concurrency through async
/// operations.
///
/// When setting the scope for the DIO it will behave differently when the commit function
/// is invoked based on what scope you set for the transaction.
pub struct Dio<'a>
where Self: Send + Sync
{
    pub(super) multi: ChainMultiUser,
    pub(super) state: DioState,
    pub(super) session: &'a Session,
    pub(super) scope: Scope,
}

impl<'a> Dio<'a>
{
    pub fn store<D>(&mut self, data: D) -> Result<Dao<D>, SerializationError>
    where D: Serialize + DeserializeOwned + Clone + Send + Sync,
    {
        self.store_ext(data, self.session.log_format, None, true)
    }

    pub fn prep<D>(&mut self, data: D) -> Result<Dao<D>, SerializationError>
    where D: Serialize + DeserializeOwned + Clone + Send + Sync,
    {
        self.store_ext(data, self.session.log_format, None, false)
    }

    pub fn store_ext<D>(&mut self, data: D, format: Option<MessageFormat>, key: Option<PrimaryKey>, auto_commit: bool) -> Result<Dao<D>, SerializationError>
    where D: Serialize + DeserializeOwned + Clone + Send + Sync,
    {
        let format = match format {
            Some(a) => a,
            None => self.multi.default_format
        };

        let row = Row {
            key: match key {
                Some(k) => k,
                None => PrimaryKey::generate(),
            },
            type_name: std::any::type_name::<D>(),
            parent: None,
            data: data,
            auth: MetaAuthorization::default(),
            collections: FxHashSet::default(),
            format,
            created: 0,
            updated: 0,
            extra_meta: Vec::new()
        };

        let mut ret = Dao::new(row);
        ret.state.dirty = true;
        if auto_commit {
            ret.commit(self)?;
        }
        Ok(ret)
    }

    pub async fn delete<D>(&mut self, key: &PrimaryKey) -> Result<(), LoadError>
    where D: Serialize + DeserializeOwned + Clone + Send + Sync,
    {
        {
            let state = &self.state;
            if state.is_locked(key) {
                return Result::Err(LoadError::ObjectStillLocked(key.clone()));
            }
            if let Some(dao) = state.cache_store_primary.get(key) {
                let row = Row::from_row_data(dao.deref())?;
                let dao = Dao::<D>::new(row);
                dao.delete(self)?;
                return Ok(());
            }
            if let Some((dao, leaf)) = state.cache_load.get(key) {
                let row = Row::from_event(dao.deref(), leaf.created, leaf.updated)?;
                let dao = Dao::<D>::new(row);
                dao.delete(self)?;
                return Ok(());
            }
            if state.deleted.contains(&key) {
                return Result::Err(LoadError::AlreadyDeleted(key.clone()));
            }
        }
        
        let parent = self.multi.lookup_parent(key).await;
        self.state.add_deleted(key.clone(), parent);
        Ok(())
    }

    pub async fn load<D>(&mut self, key: &PrimaryKey) -> Result<Dao<D>, LoadError>
    where D: Serialize + DeserializeOwned + Clone + Send + Sync,
    {
        {
            let state = &self.state;
            if state.is_locked(key) {
                return Result::Err(LoadError::ObjectStillLocked(key.clone()));
            }
            if let Some(dao) = state.cache_store_primary.get(key) {
                let row = Row::from_row_data(dao.deref())?;
                return Ok(Dao::new(row));
            }
            if let Some((dao, leaf)) = state.cache_load.get(key) {
                let row = Row::from_event(dao.deref(), leaf.created, leaf.updated)?;
                return Ok(Dao::new(row));
            }
            if state.deleted.contains(&key) {
                return Result::Err(LoadError::AlreadyDeleted(key.clone()));
            }
        }

        let entry = match self.multi.lookup_primary(key).await {
            Some(a) => a,
            None => return Result::Err(LoadError::NotFound(key.clone()))
        };

        Ok(self.load_from_entry(entry).await?)
    }

    pub async fn exists(&mut self, key: &PrimaryKey) -> bool
    {
        {
            let state = &self.state;
            if let Some(_) = state.cache_store_primary.get(key) {
                return true;
            }
            if let Some((_, _)) = state.cache_load.get(key) {
                return true;
            }
            if state.deleted.contains(&key) {
                return false;
            }
        }

        self.multi.lookup_primary(key).await.is_some()
    }

    pub(crate) async fn load_from_entry<D>(&mut self, leaf: EventLeaf)
    -> Result<Dao<D>, LoadError>
    where D: Serialize + DeserializeOwned + Clone + Send + Sync,
    {
        let evt = self.multi.load(leaf).await?;

        Ok(self.load_from_event(evt.data, evt.header.as_header()?, leaf)?)
    }

    pub(crate) fn load_from_event<D>(&mut self, mut data: EventData, header: EventHeader, leaf: EventLeaf)
    -> Result<Dao<D>, LoadError>
    where D: Serialize + DeserializeOwned + Clone + Send + Sync,
    {
        data.data_bytes = match data.data_bytes {
            Some(data) => Some(self.multi.data_as_overlay(&header.meta, data, &self.session)?),
            None => None,
        };

        let state = &mut self.state;
        match header.meta.get_data_key() {
            Some(key) => {
                let row = Row::from_event(&data, leaf.created, leaf.updated)?;
                state.cache_load.insert(key.clone(), (Arc::new(data), leaf));
                Ok(Dao::new(row))
            },
            None => Err(LoadError::NoPrimaryKey)
        }
    }

    pub(crate) async fn children<D>(&mut self, parent_id: PrimaryKey, collection_id: u64) -> Result<Vec<Dao<D>>, LoadError>
    where D: Serialize + DeserializeOwned + Clone + Send + Sync,
    {
        // Build the secondary index key
        let key = MetaCollection {
            parent_id,
            collection_id,
        };

        // This is the main return list
        let mut already = FxHashSet::default();
        let mut ret = Vec::new();

        // We either find existing objects in the cache or build a list of objects to load
        let mut to_load = Vec::new();
        for key in match self.multi.lookup_secondary_raw(&key).await {
            Some(a) => a,
            None => return Ok(Vec::new())
        } {
            {
                let state = &self.state;
                if state.is_locked(&key) {
                    return Result::Err(LoadError::ObjectStillLocked(key));
                }
                if let Some(dao) = state.cache_store_primary.get(&key) {
                    let row = Row::from_row_data(dao.deref())?;
                    already.insert(row.key.clone());
                    ret.push(Dao::new(row));
                    continue;
                }
                if let Some((dao, leaf)) = state.cache_load.get(&key) {
                    let row = Row::from_event(dao.deref(), leaf.created, leaf.updated)?;
                    already.insert(row.key.clone());
                    ret.push(Dao::new(row));
                    continue;
                }
                if state.deleted.contains(&key) {
                    continue;
                }
            }

            to_load.push(match self.multi.lookup_primary(&key).await {
                Some(a) => a,
                None => { continue },
            });
        }

        // Load all the objects that have not yet been loaded
        for mut evt in self.multi.load_many(to_load).await? {
            let mut header = evt.header.as_header()?;

            let key = match header.meta.get_data_key() {
                Some(k) => k,
                None => { continue; }
            };

            let state = &mut self.state;
            if state.is_locked(&key) {
                return Result::Err(LoadError::ObjectStillLocked(key.clone()));
            }

            if let Some(dao) = state.cache_store_primary.get(&key) {
                let row = Row::from_row_data(dao.deref())?;

                already.insert(row.key.clone());
                ret.push(Dao::new(row));
                continue;
            }
            if let Some((dao, leaf)) = state.cache_load.get(&key) {
                let row = Row::from_event(dao.deref(), leaf.created, leaf.updated)?;

                already.insert(row.key.clone());
                ret.push(Dao::new(row));
            }
            if state.deleted.contains(&key) {
                continue;
            }

            evt.data.data_bytes = match evt.data.data_bytes {
                Some(data) => Some(self.multi.data_as_overlay(&mut header.meta, data, &self.session)?),
                None => { continue; },
            };

            let row = Row::from_event(&evt.data, evt.leaf.created, evt.leaf.updated)?;
            state.cache_load.insert(row.key.clone(), (Arc::new(evt.data), evt.leaf));

            already.insert(row.key.clone());
            ret.push(Dao::new(row));
        }

        // Now we search the secondary local index so any objects we have
        // added in this transaction scope are returned
        let state = &self.state;
        if let Some(vec) = state.cache_store_secondary.get_vec(&key) {
            for a in vec {
                // This is an OR of two lists so its likely that the object
                // may already be in the return list
                if already.contains(a) {
                    continue;
                }
                if state.deleted.contains(a) {
                    continue;
                }

                // If its still locked then that is a problem
                if state.is_locked(a) {
                    return Result::Err(LoadError::ObjectStillLocked(a.clone()));
                }

                if let Some(dao) = state.cache_store_primary.get(a) {
                    let row = Row::from_row_data(dao.deref())?;
    
                    already.insert(row.key.clone());
                    ret.push(Dao::new(row));
                }
            }
        }

        Ok(ret)
    }
}

impl Chain
{
    #[allow(dead_code)]
    pub async fn dio<'a>(&'a self, session: &'a Session) -> Dio<'a> {
        self.dio_ext(session, Scope::Local).await
    }

    #[allow(dead_code)]
    pub async fn dio_ext<'a>(&'a self, session: &'a Session, scope: Scope) -> Dio<'a> {
        let multi = self.multi().await;
        Dio {
            state: DioState::new(),
            multi,
            session: session,
            scope,            
        }
    }
}

impl<'a> Dio<'a>
{
    pub fn has_uncommitted(&self) -> bool
    {
        let state = &self.state;
        if state.store.is_empty() && state.deleted.is_empty() {
            return false;
        }
        return true;
    }

    pub fn cancel(&mut self)
    {
        let state = &mut self.state;
        state.store.clear();   
        state.deleted.clear();
    }

    pub async fn commit(&mut self) -> Result<(), CommitError>
    {
        // If we have dirty records
        let state = &mut self.state;
        if state.store.is_empty() && state.deleted.is_empty() {
            return Ok(())
        }

        debug!("commit stored={} deleted={}", state.store.len(), state.deleted.len());
        
        // Declare variables
        let mut evts = Vec::new();
        let mut trans_meta = TransactionMetadata::default();

        // Determine the format of the message
        let format = match self.session.log_format {
            Some(a) => a,
            None => self.multi.default_format
        };
        
        {
            // Take all the locks we need to perform the commit actions
            let multi_lock = self.multi.lock().await;

            // Convert all the events that we are storing into serialize data
            for row in state.store.drain(..)
            {
                // Debug output
                debug!("store: {}@{}", row.type_name, row.key.as_hex_string());

                // Build a new clean metadata header
                let mut meta = Metadata::for_data(row.key);
                if row.auth.is_relevant() {
                    meta.core.push(CoreMetadata::Authorization(row.auth.clone()));
                }
                if let Some(parent) = &row.parent {
                    meta.core.push(CoreMetadata::Parent(parent.clone()))
                } else {
                    if multi_lock.inside_async.disable_new_roots == true {
                        return Err(CommitError::NewRootsAreDisabled);
                    }
                }
                for extra in row.extra_meta.iter() {
                    meta.core.push(extra.clone());
                }

                // Compute all the extra metadata for an event
                let extra_meta = multi_lock.metadata_lint_event(&mut meta, &self.session, &trans_meta)?;
                meta.core.extend(extra_meta);

                // Add the data to the transaction metadata object
                if let Some(key) = meta.get_data_key() {
                    trans_meta.auth.insert(key, match meta.get_authorization() {
                        Some(a) => a.clone(),
                        None => MetaAuthorization {
                            read: ReadOption::Inherit,
                            write: WriteOption::Inherit,
                        }
                    });
                    if let Some(parent) = meta.get_parent() {
                        trans_meta.parents.insert(key, parent.clone());
                    }
                }
                
                // Perform any transformation (e.g. data encryption and compression)
                let data = multi_lock.data_as_underlay(&mut meta, row.data.clone(), &self.session)?;
                
                // Only once all the rows are processed will we ship it to the redo log
                let evt = EventData {
                    meta: meta,
                    data_bytes: Some(data),
                    format: row.format,
                };
                evts.push(evt);
            }

            // Build events that will represent tombstones on all these records (they will be sent after the writes)
            for key in state.deleted.drain() {
                let mut meta = Metadata::default();
                meta.core.push(CoreMetadata::Authorization(MetaAuthorization {
                    read: ReadOption::Everyone,
                    write: WriteOption::Nobody,
                }));
                if let Some(parent) = multi_lock.inside_async.chain.lookup_parent(&key) {
                    meta.core.push(CoreMetadata::Parent(parent))
                }
                meta.add_tombstone(key);
                
                // Compute all the extra metadata for an event
                let extra_meta = multi_lock.metadata_lint_event(&mut meta, &self.session, &trans_meta)?;
                meta.core.extend(extra_meta);

                let evt = EventData {
                    meta: meta,
                    data_bytes: None,
                    format,
                };
                evts.push(evt);
            }

            // Lint the data
            let mut lints = Vec::new();
            for evt in evts.iter() {
                lints.push(LintData {
                    data: evt,
                    header: evt.as_header()?,
                });
            }
            let meta = multi_lock.metadata_lint_many(&lints, &self.session)?;

            // If it has data then insert it at the front of these events
            if meta.len() > 0 {
                evts.insert(0, EventData {
                    meta: Metadata {
                        core: meta,
                    },
                    data_bytes: None,
                    format,
                });
            }
        }

        // Create the transaction
        let trans = Transaction {
            scope: self.scope.clone(),
            transmit: true,
            events: evts,
        };
        debug!("commit events={}", trans.events.len());

        // Process the transaction in the chain using its pipe
        self.multi.pipe.feed(trans).await?;

        // Last thing we do is kick off an unlock operation using fire and forget
        let unlock_multi = self.multi.clone();
        let unlock_me = state.pipe_unlock.iter().map(|a| a.clone()).collect::<Vec<_>>();
        tokio::spawn(async move {
            for key in unlock_me {
                let _ = unlock_multi.pipe.unlock(key).await;
            }
        });

        // Success
        Ok(())
    }
}

impl<'a> Drop
for Dio<'a>
{
    fn drop(&mut self)
    {
        // If the DIO has uncommitted changes then warn the caller
        debug_assert!(self.has_uncommitted() == false, "dio-has-uncommitted - the DIO has uncommitted data in it - call the .commit() method before the DIO goes out of scope.");
    }
}