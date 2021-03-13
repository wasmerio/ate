mod dao;
mod vec;
mod bus;

use fxhash::{FxHashMap, FxHashSet};

use multimap::MultiMap;
#[cfg(test)]
use serde::{Deserialize};
use serde::{Serialize, de::DeserializeOwned};
use std::cell::{RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::ops::Deref;
#[allow(unused_imports)]
use tokio::sync::mpsc;
#[allow(unused_imports)]
use std::sync::mpsc as smpsc;

#[allow(unused_imports)]
use crate::crypto::{EncryptedPrivateKey, PrivateKey};
#[allow(unused_imports)]
use crate::{crypto::EncryptKey, session::{Session, SessionProperty}};

use super::header::*;
use super::multi::*;
use super::event::*;
use super::meta::*;
use super::error::*;
use super::dio::dao::*;
use super::accessor::*;
#[allow(unused_imports)]
use super::pipe::*;
#[allow(unused_imports)]
use super::crypto::*;
use super::transaction::*;

pub use crate::dio::vec::DaoVec;
pub use crate::dio::dao::Dao;

#[derive(Debug)]
pub(crate) struct DioState
{
    pub(super) store: Vec<Rc<RowData>>,
    pub(super) cache_store_primary: FxHashMap<PrimaryKey, Rc<RowData>>,
    pub(super) cache_store_secondary: MultiMap<MetaCollection, PrimaryKey>,
    pub(super) cache_load: FxHashMap<PrimaryKey, Rc<EventExt>>,
    pub(super) locked: FxHashSet<PrimaryKey>,
    pub(super) deleted: FxHashMap<PrimaryKey, Rc<RowData>>,
    pub(super) pipe_unlock: FxHashSet<PrimaryKey>,
}

impl DioState
{
    pub(super) fn dirty(&mut self, key: &PrimaryKey, tree: Option<&MetaTree>, row: RowData) {
        let row = Rc::new(row);
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

pub struct Dio<'a>
{
    multi: ChainMultiUser,
    state: Rc<RefCell<DioState>>,
    #[allow(dead_code)]
    session: &'a Session,
    scope: Scope,
}

impl<'a> Dio<'a>
{
    #[allow(dead_code)]
    pub fn store<D>(&mut self, data: D) -> Result<Dao<D>, SerializationError>
    where D: Serialize + DeserializeOwned + Clone,
    {
        let row = Row {
            key: PrimaryKey::generate(),
            tree: None,
            data: data,
            auth: MetaAuthorization::default(),
            collections: FxHashSet::default(),
        };

        let mut ret = Dao::new(row, &self.state);
        ret.fork();
        
        Ok(ret)
    }

    #[allow(dead_code)]
    pub async fn load<D>(&mut self, key: &PrimaryKey) -> Result<Dao<D>, LoadError>
    where D: Serialize + DeserializeOwned + Clone,
    {
        let state = self.state.borrow_mut();
        if state.is_locked(key) {
            return Result::Err(LoadError::ObjectStillLocked(key.clone()));
        }
        if let Some(dao) = state.cache_store_primary.get(key) {
            let row = Row::from_row_data(dao.deref())?;
            return Ok(Dao::new(row, &self.state));
        }
        if let Some(dao) = state.cache_load.get(key) {
            let row = Row::from_event(dao.deref())?;
            return Ok(Dao::new(row, &self.state));
        }
        if state.deleted.contains_key(key) {
            return Result::Err(LoadError::AlreadyDeleted(key.clone()));
        }

        let entry = match self.multi.lookup_primary(key).await {
            Some(a) => a,
            None => return Result::Err(LoadError::NotFound(key.clone()))
        };
        if entry.meta.get_tombstone().is_some() {
            return Result::Err(LoadError::Tombstoned(key.clone()));
        }
        drop(state);

        Ok(self.load_from_entry(entry).await?)
    }

    pub(crate) async fn load_from_entry<D>(&mut self, entry: EventEntryExt)
    -> Result<Dao<D>, LoadError>
    where D: Serialize + DeserializeOwned + Clone,
    {
        let evt = self.multi.load(&entry).await?;        
        Ok(self.load_from_event(evt)?)
    }

    pub(crate) fn load_from_event<D>(&mut self, mut evt: EventExt)
    -> Result<Dao<D>, LoadError>
    where D: Serialize + DeserializeOwned + Clone,
    {
        evt.raw.data = match evt.raw.data {
            Some(data) => Some(self.multi.data_as_overlay(&mut evt.raw.meta, data, &self.session)?),
            None => None,
        };

        let mut state = self.state.borrow_mut();

        match evt.raw.meta.get_data_key() {
            Some(key) => {
                let row = Row::from_event(&evt)?;
                state.cache_load.insert(key.clone(), Rc::new(evt));
                Ok(Dao::new(row, &self.state))
            },
            None => Err(LoadError::NoPrimaryKey)
        }
    }

    pub async fn children<D>(&mut self, parent_id: PrimaryKey, collection_id: u64) -> Result<Vec<Dao<D>>, LoadError>
    where D: Serialize + DeserializeOwned + Clone,
    {
        let mut state = self.state.borrow_mut();
        
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
        for entry in match self.multi.lookup_secondary(&key).await {
            Some(a) => a,
            None => return Ok(Vec::new())
        } {
            // Obviously if its tombstoned then we are done
            if entry.meta.get_tombstone().is_some() {
                continue;
            }

            let key = match entry.meta.get_data_key() {
                Some(k) => k,
                None => { continue; }
            };
            if state.is_locked(&key) {
                return Result::Err(LoadError::ObjectStillLocked(key.clone()));
            }

            if let Some(dao) = state.cache_store_primary.get(&key) {
                let row = Row::from_row_data(dao.deref())?;

                already.insert(row.key.clone());
                ret.push(Dao::new(row, &self.state));
                continue;
            }
            if let Some(dao) = state.cache_load.get(&key) {
                let row = Row::from_event(dao.deref())?;

                already.insert(row.key.clone());
                ret.push(Dao::new(row, &self.state));
            }
            if state.deleted.contains_key(&key) {
                continue;
            }
    
            to_load.push(entry);
        }

        // Load all the objects that have not yet been loaded
        for mut evt in self.multi.load_many(to_load).await? {
            evt.raw.data = match evt.raw.data {
                Some(data) => Some(self.multi.data_as_overlay(&mut evt.raw.meta, data, &self.session)?),
                None => { continue; },
            };

            let row = Row::from_event(&evt)?;
            state.cache_load.insert(row.key.clone(), Rc::new(evt));

            already.insert(row.key.clone());
            ret.push(Dao::new(row, &self.state));
        }

        // Now we search the secondary local index so any objects we have
        // added in this transaction scope are returned
        if let Some(vec) = state.cache_store_secondary.get_vec(&key) {
            for a in vec {
                // This is an OR of two lists so its likely that the object
                // may already be in the return list
                if already.contains(a) {
                    continue;
                }

                // If its still locked then that is a problem
                if state.is_locked(a) {
                    return Result::Err(LoadError::ObjectStillLocked(a.clone()));
                }

                if let Some(dao) = state.cache_store_primary.get(a) {
                    let row = Row::from_row_data(dao.deref())?;
    
                    already.insert(row.key.clone());
                    ret.push(Dao::new(row, &self.state));
                }
            }
        }

        Ok(ret)
    }
}

impl ChainAccessor
{
    #[allow(dead_code)]
    pub async fn dio<'a>(&'a self, session: &'a Session) -> Dio<'a> {
        self.dio_ext(session, Scope::Local).await
    }

    #[allow(dead_code)]
    pub async fn dio_ext<'a>(&'a self, session: &'a Session, scope: Scope) -> Dio<'a> {
        Dio {
            state: Rc::new(RefCell::new(DioState::new())),
            multi: self.multi().await,
            session: session,
            scope,
        }
    }
}



impl<'a> Drop
for Dio<'a>
{
    fn drop(&mut self)
    {
        if let Err(err) = self.commit() {
            debug_assert!(false, "dio-commit-error {}", err.to_string());
        }
    }
}

impl<'a> Dio<'a>
{
    pub fn commit(&mut self) -> Result<(), CommitError>
    {
        // If we have dirty records
        let mut state = self.state.borrow_mut();
        if state.store.is_empty() && state.deleted.is_empty() {
            return Ok(())
        }

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
            let data_hash = super::crypto::Hash::from_bytes(&data[..]);
            
            // Only once all the rows are processed will we ship it to the redo log
            let evt = EventRaw {
                meta: meta,
                data_hash: Some(data_hash),
                data: Some(data),
            }.as_plus()?;
            evts.push(evt);
        }

        // Build events that will represent tombstones on all these records (they will be sent after the writes)
        for (key, row) in &state.deleted {
            let mut meta = Metadata::default();
            meta.core.push(CoreMetadata::Authorization(row.auth.clone()));
            if let Some(tree) = &row.tree {
                meta.core.push(CoreMetadata::Tree(tree.clone()))
            }

            // Compute all the extra metadata for an event
            let extra_meta = self.multi.metadata_lint_event(&mut meta, &self.session)?;
            meta.core.extend(extra_meta);

            meta.add_tombstone(key.clone());
            let evt = EventRaw {
                meta: meta,
                data_hash: None,
                data: None,
            }.as_plus()?;
            evts.push(evt);
        }

        // Lint the data
        let meta = self.multi.metadata_lint_many(&evts, &self.session)?;

        // If it has data then insert it at the front of these events
        if meta.len() > 0 {
            evts.insert(0, EventRaw {
                meta: Metadata {
                    core: meta,
                },
                data_hash: None,
                data: None,
            }.as_plus()?);
        }

        // Create the transaction
        let (sender, receiver) = smpsc::channel();
        let trans = Transaction {
            scope: self.scope.clone(),
            events: evts,
            result: match &self.scope {
                Scope::None => None,
                _ => Some(sender)
            },
        };

        // Process it in the chain of trust
        let pipe = Arc::clone(&self.multi.pipe);
        tokio::task::spawn(async move {
            let _ = pipe.feed(trans).await;
        });
        
        // Wait for the transaction to commit (or not?) - if an error occurs it will
        // be returned to the caller
        match &self.scope {
            Scope::None => { },
            _ => {
                tokio::task::block_in_place(move || {
                    receiver.recv()
                })??
            }
        };

        // Success
        Ok(())
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
    let write_key = PrivateKey::generate(crate::crypto::KeySize::Bit192);
    let write_key2 = PrivateKey::generate(KeySize::Bit256);
    let read_key = EncryptKey::generate(crate::crypto::KeySize::Bit192);
    let root_public_key = write_key.as_public_key();
    
    let mut session = Session::default();
    let chain = super::chain::create_test_chain("test_dio".to_string(), true, false, Some(root_public_key)).await;
    //let mut chain = create_test_chain("test_dio".to_string(), true, false, None);

    session.properties.push(SessionProperty::WriteKey(write_key.clone()));
    session.properties.push(SessionProperty::WriteKey(write_key2.clone()));
    session.properties.push(SessionProperty::ReadKey(read_key.clone()));
    session.properties.push(SessionProperty::Identity("author@here.com".to_string()));

    let key1;
    let key2;
    let key3;

    // Write a value immediately from chain (this data will remain in the transaction)
    {
        let mut dio = chain.dio(&session).await;
        {
            let mut mock_dao = TestStructDao::default();
            mock_dao.val = 1;
            mock_dao.hidden = "This text should be hidden".to_string();
            
            let mut dao1 = dio.store(mock_dao).unwrap();
            let dao3 = dao1.inner.push(&mut dio, dao1.key(), TestEnumDao::Blah1).unwrap();

            key1 = dao1.key().clone();
            println!("key1: {}", key1.as_hex_string());

            key3 = dao3.key().clone();
            println!("key3: {}", key3.as_hex_string());
            
            dio.load::<TestStructDao>(&key1).await.expect_err("This load is meant to fail as we are still editing the object");

            dao1.auth_mut().read = ReadOption::Specific(read_key.hash());
            dao1.auth_mut().write = WriteOption::Specific(write_key2.hash());
        }   
    }

    {
        let mut dio = chain.dio(&session).await;
        {
            // Load the object again which should load it from the cache
            let mut dao1 = dio.load::<TestStructDao>(&key1).await.unwrap();

            // When we update this value it will become dirty and hence should block future loads until its flushed or goes out of scope
            dao1.val = 2;
            dio.load::<TestStructDao>(&key1).await.expect_err("This load is meant to fail due to a lock being triggered");

            // Flush the data and attempt to read it again (this should succeed)
            dao1.flush().expect("Flush failed");
            let test: Dao<TestStructDao> = dio.load(&key1).await.expect("The dirty data object should have been read after it was flushed");
            assert_eq!(test.val, 2 as u32);
        }

        {
            // Load the object again which should load it from the cache
            let mut dao1 = dio.load::<TestStructDao>(&key1).await.unwrap();
        
            // Again after changing the data reads should fail
            dao1.val = 3;
            dio.load::<TestStructDao>(&key1).await.expect_err("This load is meant to fail due to a lock being triggered");
        }

        {
            // Write a record to the chain that we will delete again later
            let mut dao2 = dio.store(TestEnumDao::Blah4).unwrap();
            
            // We create a new private key for this data
            dao2.auth_mut().write = WriteOption::Specific(write_key2.as_public_key().hash());
            
            key2 = dao2.key().clone();
            println!("key2: {}", key2.as_hex_string());
        }
    }

    {
        let mut dio = chain.dio(&session).await;
        
        // Now its out of scope it should be loadable again
        let test = dio.load::<TestStructDao>(&key1).await.expect("The dirty data object should have been read after it was flushed");
        assert_eq!(test.val, 3);

        // Read the items in the collection which we should find our second object
        let test3 = test.inner.iter(test.key(), &mut dio).await.unwrap().next().expect("Three should be a data object in this collection");
        assert_eq!(test3.key(), &key3);
    }

    {
        let mut dio = chain.dio(&session).await;

        // The data we saved earlier should be accessible accross DIO scope boundaries
        let mut dao1: Dao<TestStructDao> = dio.load(&key1).await.expect("The data object should have been read");
        assert_eq!(dao1.val, 3);
        dao1.val = 4;

        // First attempt to read the record then delete it
        let dao2 = dio.load::<TestEnumDao>(&key2).await.expect("The record should load before we delete it in this session");
        dao2.delete().unwrap();

        // It should no longer load now that we deleted it
        dio.load::<TestEnumDao>(&key2).await.expect_err("This load should fail as we deleted the record");
    }

    {
        let mut dio = chain.dio(&session).await;

        // After going out of scope then back again we should still no longer see the record we deleted
        dio.load::<TestEnumDao>(&key2).await.expect_err("This load should fail as we deleted the record");
    }

    //chain.single().destroy().unwrap();
}