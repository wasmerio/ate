use fxhash::{FxHashMap, FxHashSet};

use multimap::MultiMap;
#[cfg(test)]
use serde::{Deserialize};
use serde::{Serialize, de::DeserializeOwned};
use bytes::Bytes;
use std::cell::{RefCell};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

#[allow(unused_imports)]
use crate::crypto::{EncryptedPrivateKey, PrivateKey};
#[allow(unused_imports)]
use crate::{crypto::EncryptKey, session::{Session, SessionProperty}};

use super::header::*;
use super::chain::*;
use super::event::*;
use super::meta::*;
use super::error::*;
#[allow(unused_imports)]
use super::crypto::*;

pub use super::collection::DaoVec;
pub use super::collection::DaoVecExt;

#[allow(dead_code)]
type Dio<'a> = DioExt<'a, NoAdditionalMetadata>;
#[allow(dead_code)]
type Dao<D> = DaoExt<NoAdditionalMetadata, D>;

#[derive(Debug, Clone)]
pub(super) struct Row<M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
{
    pub(super) key: PrimaryKey,
    pub(super) tree: Option<MetaTree>,
    pub(super) meta: M,
    pub(super) data: D,
    pub(super) auth: MetaAuthorization,
    pub(super) collections: FxHashSet<MetaCollection>,
}

impl<D, M> Row<M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
{
    #[allow(dead_code)]
    pub(super) fn new(
        key: PrimaryKey,
        meta: M,
        data: D,
        auth: MetaAuthorization,
        tree: Option<MetaTree>,
        collections: FxHashSet<MetaCollection>,
    ) -> Row<M, D>
    {
        Row {
            key,
            tree,
            meta,
            data,
            auth,
            collections,
        }
    }

    pub fn from_event(evt: &EventExt<M>) -> Result<Row<M, D>, SerializationError> {
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
                        meta: evt.raw.meta.other.clone(),
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

    pub fn from_row_data(row: &RowData<M>) -> Result<Row<M, D>, SerializationError> {
        Ok(
            Row {
                key: row.key,
                tree: row.tree.clone(),
                meta: row.meta.clone(),
                data: serde_json::from_slice(&row.data)?,
                auth: row.auth.clone(),
                collections: row.collections.clone(),
            }
        )
    }

    pub fn as_row_data(&self) -> std::result::Result<RowData<M>, SerializationError> {
        let data = Bytes::from(serde_json::to_vec(&self.data)?);
        let data_hash = super::crypto::Hash::from_bytes(&data[..]);
        Ok
        (
            RowData {
                key: self.key.clone(),
                tree: self.tree.clone(),
                meta: self.meta.clone(),
                data_hash,
                data,
                auth: self.auth.clone(),
                collections: self.collections.clone(),
            }
        )
    }
}

#[derive(Debug, Clone)]
pub(super) struct RowData<M>
where M: OtherMetadata
{
    pub key: PrimaryKey,
    pub tree: Option<MetaTree>,
    pub meta: M,
    pub data_hash: super::crypto::Hash,
    pub data: Bytes,
    pub auth: MetaAuthorization,
    pub collections: FxHashSet<MetaCollection>,
}

#[derive(Debug)]
pub struct DaoExt<M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
{
    dirty: bool,
    pub(super) row: Row<M, D>,
    state: Rc<RefCell<DioState<M>>>,
}

impl<M, D> DaoExt<M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
{
    fn new<>(row: Row<M, D>, state: &Rc<RefCell<DioState<M>>>) -> DaoExt<M, D> {
        DaoExt {
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

    pub fn flush(&mut self) -> std::result::Result<(), SerializationError> {
        if self.dirty == true
        {            
            let mut state = self.state.borrow_mut();
            state.unlock(&self.row.key);
            
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

    #[allow(dead_code)]
    pub fn key(&self) -> &PrimaryKey {
        &self.row.key
    }

    #[allow(dead_code)]
    pub fn metadata(&self) -> &M {
        &self.row.meta
    }

    #[allow(dead_code)]
    pub fn detach(&mut self) {
        self.fork();
        self.row.tree = None;
    }

    #[allow(dead_code)]
    pub fn metadata_mut(&mut self) -> &mut M {
        self.fork();
        &mut self.row.meta
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
    pub fn delete(self) -> std::result::Result<(), SerializationError> {
        let mut state = self.state.borrow_mut();
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
}

impl<M, D> Deref for DaoExt<M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
{
    type Target = D;

    fn deref(&self) -> &Self::Target {
        &self.row.data
    }
}

impl<M, D> DerefMut for DaoExt<M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.fork();
        &mut self.row.data
    }
}

impl<M, D> Drop for DaoExt<M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
{
    fn drop(&mut self)
    {
        // Now attempt to flush it
        if let Err(err) = self.flush() {
            debug_assert!(false, "dao-flush-error {}", err.to_string());
        }
    }
}

#[derive(Debug)]
struct DioState<M>
where M: OtherMetadata,
{
    store: Vec<Rc<RowData<M>>>,
    cache_store_primary: FxHashMap<PrimaryKey, Rc<RowData<M>>>,
    cache_store_secondary: MultiMap<MetaCollection, PrimaryKey>,
    cache_load: FxHashMap<PrimaryKey, Rc<EventExt<M>>>,
    locked: FxHashSet<PrimaryKey>,
    deleted: FxHashMap<PrimaryKey, Rc<RowData<M>>>,
}

impl<M> DioState<M>
where M: OtherMetadata,
{
    fn dirty(&mut self, key: &PrimaryKey, tree: Option<&MetaTree>, row: RowData<M>) {
        let row = Rc::new(row);
        self.store.push(row.clone());
        self.cache_store_primary.insert(key.clone(), row);
        if let Some(tree) = tree {
            self.cache_store_secondary.insert(tree.vec.clone(), key.clone());
        }
        self.cache_load.remove(key);
    }

    fn lock(&mut self, key: &PrimaryKey) -> bool {
        self.locked.insert(key.clone())
    }

    fn unlock(&mut self, key: &PrimaryKey) -> bool {
        self.locked.remove(key)
    }

    fn is_locked(&self, key: &PrimaryKey) -> bool {
        self.locked.contains(key)
    }
}

impl<M> DioState<M>
where M: OtherMetadata,
{
    fn new() -> DioState<M> {
        DioState {
            store: Vec::new(),
            cache_store_primary: FxHashMap::default(),
            cache_store_secondary: MultiMap::new(),
            cache_load: FxHashMap::default(),
            locked: FxHashSet::default(),
            deleted: FxHashMap::default(),
        }
    }
}

pub struct DioExt<'a, M>
where M: OtherMetadata,
{
    accessor: ChainAccessorExt<M>,
    multi: Option<ChainMultiUserExt<'a, M>>,
    state: Rc<RefCell<DioState<M>>>,
    #[allow(dead_code)]
    session: &'a Session,
}

impl<'a, M> DioExt<'a, M>
where M: OtherMetadata,
{
    #[allow(dead_code)]
    pub fn store<D>(&mut self, data: D) -> Result<DaoExt<M, D>, SerializationError>
    where D: Serialize + DeserializeOwned + Clone,
    {
        let row = Row {
            key: PrimaryKey::generate(),
            tree: None,
            meta: M::default(),
            data: data,
            auth: MetaAuthorization::default(),
            collections: FxHashSet::default(),
        };

        let mut ret = DaoExt::new(row, &self.state);
        ret.fork();
        
        Ok(ret)
    }

    #[allow(dead_code)]
    pub fn load<D>(&mut self, key: &PrimaryKey) -> Result<DaoExt<M, D>, LoadError>
    where D: Serialize + DeserializeOwned + Clone,
    {
        let mut state = self.state.borrow_mut();
        if state.is_locked(key) {
            return Result::Err(LoadError::ObjectStillLocked(key.clone()));
        }
        if let Some(dao) = state.cache_store_primary.get(key) {
            let row = Row::from_row_data(dao.deref())?;
            return Ok(DaoExt::new(row, &self.state));
        }
        if let Some(dao) = state.cache_load.get(key) {
            let row = Row::from_event(dao.deref())?;
            return Ok(DaoExt::new(row, &self.state));
        }
        if state.deleted.contains_key(key) {
            return Result::Err(LoadError::AlreadyDeleted(key.clone()));
        }

        let multi_store;
        let multi = match self.multi.as_ref() {
            Some(a) => a,
            None => {
                multi_store = self.accessor.multi();
                &multi_store
            }
        };

        let entry = match multi.lookup_primary(key) {
            Some(a) => a,
            None => return Result::Err(LoadError::NotFound(key.clone()))
        };
        if entry.meta.get_tombstone().is_some() {
            return Result::Err(LoadError::Tombstoned(key.clone()));
        }

        let mut evt = multi.load(&entry)?;
        evt.raw.data = match evt.raw.data {
            Some(data) => Some(multi.data_as_overlay(&mut evt.raw.meta, data, &self.session)?),
            None => None,
        };

        let row = Row::from_event(&evt)?;
        state.cache_load.insert(key.clone(), Rc::new(evt));
        Ok(DaoExt::new(row, &self.state))
    }

    pub fn children<D>(&mut self, parent_id: PrimaryKey, collection_id: u64) -> Result<Vec<DaoExt<M, D>>, LoadError>
    where D: Serialize + DeserializeOwned + Clone,
    {
        let mut state = self.state.borrow_mut();
        
        // Build the secondary index key
        let key = MetaCollection {
            parent_id,
            collection_id,
        };

        // We need a multi-user access object so we can load objects later
        let multi_store;
        let multi = match self.multi.as_ref() {
            Some(a) => a,
            None => {
                multi_store = self.accessor.multi();
                &multi_store
            }
        };

        // This is the main return list
        let mut already = FxHashSet::default();
        let mut ret = Vec::new();

        // We either find existing objects in the cache or build a list of objects to load
        let mut to_load = Vec::new();
        for entry in match multi.lookup_secondary(&key) {
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
                ret.push(DaoExt::new(row, &self.state));
                continue;
            }
            if let Some(dao) = state.cache_load.get(&key) {
                let row = Row::from_event(dao.deref())?;

                already.insert(row.key.clone());
                ret.push(DaoExt::new(row, &self.state));
            }
            if state.deleted.contains_key(&key) {
                continue;
            }
    
            to_load.push(entry);
        }

        // Load all the objects that have not yet been loaded
        for mut evt in multi.load_many(to_load)? {
            evt.raw.data = match evt.raw.data {
                Some(data) => Some(multi.data_as_overlay(&mut evt.raw.meta, data, &self.session)?),
                None => { continue; },
            };

            let row = Row::from_event(&evt)?;
            state.cache_load.insert(row.key.clone(), Rc::new(evt));

            already.insert(row.key.clone());
            ret.push(DaoExt::new(row, &self.state));
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
                    ret.push(DaoExt::new(row, &self.state));
                }
            }
        }

        Ok(ret)
    }

    fn flush(&mut self) -> Result<(), FlushError>
    {
        // If we have dirty records
        let mut state = self.state.borrow_mut();
        if state.store.is_empty() == false || state.deleted.is_empty() == false
        {
            let mut evts = Vec::new();
            {
                // Take the reference to the multi for a limited amount of time then destruct it and release the lock
                let multi = match self.multi.take() {
                    Some(a) => a,
                    None => self.accessor.multi()
                };

                // Convert all the events that we are storing into serialize data
                for row in state.store.drain(..)
                {
                    // Build a new clean metadata header
                    let mut meta = MetadataExt::for_data(row.key);
                    meta.other = row.meta.clone();
                    meta.core.push(CoreMetadata::Authorization(row.auth.clone()));
                    if let Some(tree) = &row.tree {
                        meta.core.push(CoreMetadata::Tree(tree.clone()))
                    }

                    // Compute all the extra metadata for an event
                    let extra_meta = multi.metadata_lint_event(&mut meta, &self.session)?;
                    meta.core.extend(extra_meta);
                    
                    // Perform any transformation (e.g. data encryption and compression)
                    let data = multi.data_as_underlay(&mut meta, row.data.clone(), &self.session)?;
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
                    let mut meta = MetadataExt::default();
                    meta.core.push(CoreMetadata::Authorization(row.auth.clone()));
                    if let Some(tree) = &row.tree {
                        meta.core.push(CoreMetadata::Tree(tree.clone()))
                    }

                    // Compute all the extra metadata for an event
                    let extra_meta = multi.metadata_lint_event(&mut meta, &self.session)?;
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
                let meta = multi.metadata_lint_many(&evts, &self.session)?;

                // If it has data then insert it at the front of these events
                if meta.len() > 0 {
                    evts.insert(0, EventRaw {
                        meta: MetadataExt {
                            core: meta,
                            other: M::default(),
                        },
                        data_hash: None,
                        data: None,
                    }.as_plus()?);
                }
            }

            // Process it in the chain of trust
            let mut single = self.accessor.single();
            single.event_feed(evts)?;
        }

        Ok(())
    }
}

impl<'a, M> Drop
for DioExt<'a, M>
where M: OtherMetadata,
{
    fn drop(&mut self)
    {
        if let Err(err) = self.flush() {
            debug_assert!(false, "dio-flush-error {}", err.to_string());
        }
    }
}

pub trait DioFactoryExt<'a, M>
where M: OtherMetadata,
{
    fn dio(&'a mut self, session: &'a Session) -> DioExt<'a, M>;
}

impl<'a, M> DioFactoryExt<'a, M>
for ChainAccessorExt<M>
where M: OtherMetadata,
{
    fn dio(&'a mut self, session: &'a Session) -> DioExt<'a, M> {
        let accessor = ChainAccessorExt::from_accessor(self); 
        let multi = self.multi();
        DioExt {
            accessor: accessor,
            state: Rc::new(RefCell::new(DioState::new())),
            multi: Some(multi),          
            session: session,
        }
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

#[test]
fn test_dio()
{
    let write_key = PrivateKey::generate(crate::crypto::KeySize::Bit192);
    let write_key2 = PrivateKey::generate(KeySize::Bit256);
    let read_key = EncryptKey::generate(crate::crypto::KeySize::Bit192);
    let root_public_key = write_key.as_public_key();
    
    let mut session = Session::default();
    let mut chain = create_test_chain("test_dio".to_string(), true, false, Some(root_public_key));
    //let mut chain = create_test_chain("test_dio".to_string(), true, false, None);

    session.properties.push(SessionProperty::WriteKey(write_key.clone()));
    session.properties.push(SessionProperty::WriteKey(write_key2.clone()));
    session.properties.push(SessionProperty::ReadKey(read_key.clone()));
    session.properties.push(SessionProperty::Identity("author@here.com".to_string()));

    let key1;
    let key2;
    let key3;

    {
        let mut dio = chain.dio(&session);

        // Write a value immediately from chain (this data will remain in the transaction)
        {
            {
                let mut mock_dao = TestStructDao::default();
                mock_dao.val = 1;
                mock_dao.hidden = "This text should be hidden".to_string();
                
                let mut dao1 = dio.store(mock_dao).unwrap();
                let dao3 = dao1.inner.push(&mut dio, &dao1, TestEnumDao::Blah1).unwrap();

                key1 = dao1.key().clone();
                println!("key1: {}", key1.as_hex_string());

                key3 = dao3.key().clone();
                println!("key3: {}", key3.as_hex_string());
                
                dio.load::<TestStructDao>(&key1).expect_err("This load is meant to fail as we are still editing the object");

                dao1.auth_mut().allow_read = ReadOption::Specific(read_key.hash());
                dao1.auth_mut().allow_write = WriteOption::Specific(write_key2.hash());
            }

            dio.flush().unwrap();

            {
                // Load the object again which should load it from the cache
                let mut dao1 = dio.load::<TestStructDao>(&key1).unwrap();

                // When we update this value it will become dirty and hence should block future loads until its flushed or goes out of scope
                dao1.val = 2;
                dio.load::<TestStructDao>(&key1).expect_err("This load is meant to fail due to a lock being triggered");

                // Flush the data and attempt to read it again (this should succeed)
                dao1.flush().expect("Flush failed");
                let test: Dao<TestStructDao> = dio.load(&key1).expect("The dirty data object should have been read after it was flushed");
                assert_eq!(test.val, 2 as u32);
            }

            {
                // Load the object again which should load it from the cache
                let mut dao1 = dio.load::<TestStructDao>(&key1).unwrap();
            
                // Again after changing the data reads should fail
                dao1.val = 3;
                dio.load::<TestStructDao>(&key1).expect_err("This load is meant to fail due to a lock being triggered");
            }

            {
                // Write a record to the chain that we will delete again later
                let mut dao2 = dio.store(TestEnumDao::Blah4).unwrap();
                
                // We create a new private key for this data
                dao2.auth_mut().allow_write = WriteOption::Specific(write_key2.as_public_key().hash());
                
                key2 = dao2.key().clone();
                println!("key2: {}", key2.as_hex_string());
            }
        }

        // Now its out of scope it should be loadable again
        let test = dio.load::<TestStructDao>(&key1).expect("The dirty data object should have been read after it was flushed");
        assert_eq!(test.val, 3);

        // Read the items in the collection which we should find our second object
        let test3 = test.inner.iter(&test, &mut dio).unwrap().next().expect("Three should be a data object in this collection");
        assert_eq!(test3.key(), &key3);
    }

    {
        let mut dio = chain.dio(&session);

        // The data we saved earlier should be accessible accross DIO scope boundaries
        let mut dao1: Dao<TestStructDao> = dio.load(&key1).expect("The data object should have been read");
        assert_eq!(dao1.val, 3);
        dao1.val = 4;

        // First attempt to read the record then delete it
        let dao2 = dio.load::<TestEnumDao>(&key2).expect("The record should load before we delete it in this session");
        dao2.delete().unwrap();

        // It should no longer load now that we deleted it
        dio.load::<TestEnumDao>(&key2).expect_err("This load should fail as we deleted the record");
    }

    {
        let mut dio = chain.dio(&session);

        // After going out of scope then back again we should still no longer see the record we deleted
        dio.load::<TestEnumDao>(&key2).expect_err("This load should fail as we deleted the record");
    }

    //chain.single().destroy().unwrap();
}