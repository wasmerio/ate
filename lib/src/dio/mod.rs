#![allow(unused_imports)]
use log::{info, error, debug};

mod dao;
mod vec;
mod obj;
mod bus;
mod foreign;

use fxhash::{FxHashMap, FxHashSet};

use multimap::MultiMap;
use serde::{Deserialize};
use serde::{Serialize, de::DeserializeOwned};
use std::{fmt::Debug, sync::Arc};
use parking_lot::Mutex;
use std::ops::Deref;
use tokio::sync::mpsc;
use std::sync::mpsc as smpsc;

use crate::crypto::{EncryptedPrivateKey, PrivateSignKey};
use crate::{crypto::EncryptKey, session::{Session, SessionProperty}};

use super::header::*;
use super::multi::*;
use super::event::*;
use super::meta::*;
use super::lint::*;
use super::spec::*;
use super::error::*;
use super::dio::dao::*;
use super::trust::*;
use super::chain::*;
use super::pipe::*;
use super::crypto::*;
use super::transaction::*;
use super::index::*;

pub use crate::dio::vec::DaoVec;
pub use crate::dio::dao::Dao;
pub use crate::dio::obj::DaoRef;
pub use crate::dio::foreign::DaoForeign;

#[derive(Debug)]
pub(crate) struct DioState
where Self: Send + Sync
{
    pub(super) store: Vec<Arc<RowData>>,
    pub(super) cache_store_primary: FxHashMap<PrimaryKey, Arc<RowData>>,
    pub(super) cache_store_secondary: MultiMap<MetaCollection, PrimaryKey>,
    pub(super) cache_load: FxHashMap<PrimaryKey, (Arc<EventData>, EventLeaf)>,
    pub(super) locked: FxHashSet<PrimaryKey>,
    pub(super) deleted: FxHashMap<PrimaryKey, RowData>,
    pub(super) pipe_unlock: FxHashSet<PrimaryKey>,
}

impl DioState
{
    pub(super) fn dirty(&mut self, key: &PrimaryKey, tree: Option<&MetaTree>, row: RowData) {
        let row = Arc::new(row);
        self.store.push(row.clone());
        self.cache_store_primary.insert(key.clone(), row);
        if let Some(tree) = tree {
            self.cache_store_secondary.insert(tree.vec.clone(), key.clone());
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
            deleted: FxHashMap::default(),
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
    multi: ChainMultiUser,
    state: DioState,
    #[allow(dead_code)]
    session: &'a Session,
    scope: Scope,
    default_format: MessageFormat,
}

impl<'a> Dio<'a>
{
    #[allow(dead_code)]
    pub fn store<D>(&mut self, data: D) -> Result<Dao<D>, SerializationError>
    where D: Serialize + DeserializeOwned + Clone + Send + Sync,
    {
        self.store_ext(data, None, None, true)
    }

    #[allow(dead_code)]
    pub fn store_ext<D>(&mut self, data: D, format: Option<MessageFormat>, key: Option<PrimaryKey>, auto_commit: bool) -> Result<Dao<D>, SerializationError>
    where D: Serialize + DeserializeOwned + Clone + Send + Sync,
    {
        let row = Row {
            key: match key {
                Some(k) => k,
                None => PrimaryKey::generate(),
            },
            tree: None,
            data: data,
            auth: MetaAuthorization::default(),
            collections: FxHashSet::default(),
            format: match format {
                Some(f) => f,
                None => self.default_format
            },
            created: 0,
            updated: 0,
        };

        let mut ret = Dao::new(row);
        ret.state.dirty = true;
        if auto_commit {
            ret.commit(self)?;
        }
        Ok(ret)
    }

    #[allow(dead_code)]
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
            if state.deleted.contains_key(key) {
                return Result::Err(LoadError::AlreadyDeleted(key.clone()));
            }
        }

        let entry = match self.multi.lookup_primary(key).await {
            Some(a) => a,
            None => return Result::Err(LoadError::NotFound(key.clone()))
        };

        Ok(self.load_from_entry(entry).await?)
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
                if state.deleted.contains_key(&key) {
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
            if state.deleted.contains_key(&key) {
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
                if state.deleted.contains_key(a) {
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
            default_format: multi.default_format,
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

        debug!("atefs::commit stored={} deleted={}", state.store.len(), state.deleted.len());

        // First unlock any data objects that were locked via the pipe
        let unlock_multi = self.multi.clone();
        let unlock_me = state.pipe_unlock.iter().map(|a| a.clone()).collect::<Vec<_>>();
        tokio::spawn(async move {
            for key in unlock_me {
                let _ = unlock_multi.pipe.unlock(key).await;
            }
        });
        
        let mut evts = Vec::new();

        // Convert all the events that we are storing into serialize data
        for row in state.store.drain(..)
        {
            // Build a new clean metadata header
            let mut meta = Metadata::for_data(row.key);
            meta.core.push(CoreMetadata::Authorization(row.auth.clone()));
            if let Some(tree) = &row.tree {
                meta.core.push(CoreMetadata::Tree(tree.clone()))
            }

            // Compute all the extra metadata for an event
            let extra_meta = self.multi.metadata_lint_event(&mut meta, &self.session)?;
            meta.core.extend(extra_meta);
            
            // Perform any transformation (e.g. data encryption and compression)
            let data = self.multi.data_as_underlay(&mut meta, row.data.clone(), &self.session)?;
            
            // Only once all the rows are processed will we ship it to the redo log
            let evt = EventData {
                meta: meta,
                data_bytes: Some(data),
                format: self.default_format,
            };
            evts.push(evt);
        }

        // Build events that will represent tombstones on all these records (they will be sent after the writes)
        for (key, row) in state.deleted.drain() {
            let mut meta = Metadata::default();
            meta.core.push(CoreMetadata::Authorization(row.auth));
            if let Some(tree) = row.tree {
                meta.core.push(CoreMetadata::Tree(tree))
            }

            // Compute all the extra metadata for an event
            let extra_meta = self.multi.metadata_lint_event(&mut meta, &self.session)?;
            meta.core.extend(extra_meta);

            meta.add_tombstone(key);
            let evt = EventData {
                meta: meta,
                data_bytes: None,
                format: self.default_format,
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
        let meta = self.multi.metadata_lint_many(&lints, &self.session)?;

        // If it has data then insert it at the front of these events
        if meta.len() > 0 {
            evts.insert(0, EventData {
                meta: Metadata {
                    core: meta,
                },
                data_bytes: None,
                format: self.default_format,
            });
        }

        // Create the transaction
        let (sender, mut receiver) = mpsc::channel(1);
        let trans = Transaction {
            scope: self.scope.clone(),
            events: evts,
            result: match &self.scope {
                Scope::None => None,
                _ => Some(sender)
            },
        };
        debug!("atefs::commit events={}", trans.events.len());

        // Process it in the chain of trust
        self.multi.pipe.feed(trans).await?;
        
        // Wait for the transaction to commit (or not?) - if an error occurs it will
        // be returned to the caller
        match &self.scope {
            Scope::None => { },
            _ => match receiver.recv().await {
                Some(a) => a?,
                None => { return Err(CommitError::Aborted); }
            }
        };

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

#[cfg(test)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TestEnumDao
{
    None,
    Blah1,
    Blah2(u32),
    Blah3(String),
    Blah4,
    Blah5,
}

#[cfg(test)]
impl Default
for TestEnumDao
{
    fn default() -> TestEnumDao {
        TestEnumDao::None
    }
}

#[cfg(test)]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TestStructDao
{
    val: u32,
    hidden: String,
    inner: DaoVec<TestEnumDao>,
}

#[tokio::main]
#[test]
async fn test_dio()
{
    //env_logger::init();

    debug!("generating crypto keys");
    let write_key = PrivateSignKey::generate(crate::crypto::KeySize::Bit192);
    let write_key2 = PrivateSignKey::generate(KeySize::Bit256);
    let read_key = EncryptKey::generate(crate::crypto::KeySize::Bit192);
    let root_public_key = write_key.as_public_key();
    
    debug!("building the session");
    let mut session = Session::default();    
    session.properties.push(SessionProperty::WriteKey(write_key.clone()));
    session.properties.push(SessionProperty::WriteKey(write_key2.clone()));
    session.properties.push(SessionProperty::ReadKey(read_key.clone()));
    session.properties.push(SessionProperty::Identity("author@here.com".to_string()));

    let key1;
    let key2;
    let key3;
    let chain_name;

    {
        debug!("creating the chain-of-trust");
        let chain = super::trust::create_test_chain("test_dio".to_string(), true, false, Some(root_public_key.clone())).await;
        //let mut chain = create_test_chain("test_dio".to_string(), true, false, None);
        chain_name = chain.name().await.clone();

        // Write a value immediately from chain (this data will remain in the transaction)
        {
            let mut dio = chain.dio(&session).await;
            {
                debug!("storing data object 1");
                let mut mock_dao = TestStructDao::default();
                mock_dao.val = 1;
                mock_dao.hidden = "This text should be hidden".to_string();
                
                let mut dao1 = dio.store(mock_dao).unwrap();
                let mut dao3 = dao1.inner.push(&mut dio, dao1.key(), TestEnumDao::Blah1).unwrap();

                key1 = dao1.key().clone();
                debug!("key1: {}", key1.as_hex_string());

                key3 = dao3.key().clone();
                debug!("key3: {}", key3.as_hex_string());
                
                debug!("loading data object 1");
                
                debug!("setting read and write crypto keys");
                dao1.auth_mut().read = ReadOption::Specific(read_key.hash());
                dao1.auth_mut().write = WriteOption::Specific(write_key2.hash());

                dao1.commit(&mut dio).unwrap();
                dao3.commit(&mut dio).unwrap();
            }
            dio.commit().await.unwrap();
        }

        {
            debug!("new DIO context");
            let mut dio = chain.dio(&session).await;
            {
                // Load the object again which should load it from the cache
                debug!("loading data object 1");
                let mut dao1 = dio.load::<TestStructDao>(&key1).await.unwrap();

                // When we update this value it will become dirty and hence should block future loads until its flushed or goes out of scope
                debug!("updating data object");
                dao1.val = 2;
                dao1.commit(&mut dio).unwrap();

                // Flush the data and attempt to read it again (this should succeed)
                debug!("load the object again");
                let test: Dao<TestStructDao> = dio.load(&key1).await.expect("The dirty data object should have been read after it was flushed");
                assert_eq!(test.val, 2 as u32);
            }

            {
                // Load the object again which should load it from the cache
                debug!("loading data object 1 in new scope");
                let mut dao1 = dio.load::<TestStructDao>(&key1).await.unwrap();
            
                // Again after changing the data reads should fail
                debug!("modifying data object 1");
                dao1.val = 3;
                dao1.commit(&mut dio).unwrap();
            }

            {
                // Write a record to the chain that we will delete again later
                debug!("storing data object 2");
                let mut dao2 = dio.store(TestEnumDao::Blah4).unwrap();
                
                // We create a new private key for this data
                debug!("adding a write crypto key");
                dao2.auth_mut().write = WriteOption::Specific(write_key2.as_public_key().hash());
                dao2.commit(&mut dio).unwrap();
                
                key2 = dao2.key().clone();
                debug!("key2: {}", key2.as_hex_string());
            }
            dio.commit().await.expect("The DIO should commit");
        }

        {
            debug!("new DIO context");
            let mut dio = chain.dio(&session).await;
            
            // Now its out of scope it should be loadable again
            debug!("loading data object 1");
            let test = dio.load::<TestStructDao>(&key1).await.expect("The dirty data object should have been read after it was flushed");
            assert_eq!(test.val, 3);

            // Read the items in the collection which we should find our second object
            debug!("loading children");
            let test3 = test.inner.iter(test.key(), &mut dio).await.unwrap().next().expect("Three should be a data object in this collection");
            assert_eq!(test3.key(), &key3);
        }

        {
            debug!("new DIO context");
            let mut dio = chain.dio(&session).await;

            // The data we saved earlier should be accessible accross DIO scope boundaries
            debug!("loading data object 1");
            let mut dao1: Dao<TestStructDao> = dio.load(&key1).await.expect("The data object should have been read");
            assert_eq!(dao1.val, 3);
            dao1.val = 4;
            dao1.commit(&mut dio).unwrap();

            // First attempt to read the record then delete it
            debug!("loading data object 2");
            let dao2 = dio.load::<TestEnumDao>(&key2).await.expect("The record should load before we delete it in this session");

            debug!("deleting data object 2");
            dao2.delete(&mut dio).unwrap();

            // It should no longer load now that we deleted it
            debug!("negative test on loading data object 2");
            dio.load::<TestEnumDao>(&key2).await.expect_err("This load should fail as we deleted the record");

            dio.commit().await.expect("The DIO should commit");
        }
    }

    {
        debug!("reloading the chain of trust");
        let chain = super::trust::create_test_chain(chain_name.clone(), false, false, Some(root_public_key.clone())).await;

        {
            let mut dio = chain.dio(&session).await;

            // Load it again
            debug!("loading data object 1");
            let dao1: Dao<TestStructDao> = dio.load(&key1).await.expect("The data object should have been read");
            assert_eq!(dao1.val, 4);

            // After going out of scope then back again we should still no longer see the record we deleted
            debug!("loading data object 2");
            dio.load::<TestEnumDao>(&key2).await.expect_err("This load should fail as we deleted the record");
        }

        debug!("destroying the chain of trust");
        chain.single().await.destroy().await.unwrap();
    }
}