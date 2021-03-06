use fxhash::{FxHashMap, FxHashSet};

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

#[allow(dead_code)]
type Dio<'a> = DioExt<'a, NoAdditionalMetadata>;
#[allow(dead_code)]
type Dao<D> = DaoExt<NoAdditionalMetadata, D>;

#[derive(Debug, Clone)]
struct Row<M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
{
    pub key: PrimaryKey,
    pub meta: M,
    pub data: D,
    pub auth: MetaAuthorization,
}

impl<D, M> Row<M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
{
    #[allow(dead_code)]
    pub fn new(key: PrimaryKey, meta: M, data: D, auth: MetaAuthorization) -> Row<M, D> {
        Row {
            key: key,
            meta: meta,
            data: data,
            auth: auth,
        }
    }

    pub fn from_event(evt: &EventExt<M>) -> Result<Row<M, D>, SerializationError> {
        let key = match evt.raw.meta.get_data_key() {
            Some(key) => key,
            None => { return Result::Err(SerializationError::NoPrimarykey) }
        };
        match &evt.raw.data {
            Some(data) => {
                Ok(
                    Row {
                        key: key,
                        meta: evt.raw.meta.other.clone(),
                        data: serde_json::from_slice(&data)?,
                        auth: match evt.raw.meta.get_authorization() {
                            Some(a) => a.clone(),
                            None => MetaAuthorization::default(),
                        }
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
                meta: row.meta.clone(),
                data: serde_json::from_slice(&row.data)?,
                auth: row.auth.clone(),
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
                meta: self.meta.clone(),
                data_hash: data_hash,
                data: data,
                auth: self.auth.clone(),
            }
        )
    }
}

#[derive(Debug, Clone)]
struct RowData<M>
where M: OtherMetadata
{
    pub key: PrimaryKey,
    pub meta: M,
    pub data_hash: super::crypto::Hash,
    pub data: Bytes,
    pub auth: MetaAuthorization,
}

#[derive(Debug)]
pub struct DaoExt<M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
{
    dirty: bool,
    row: Row<M, D>,
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

    fn fork(&mut self) -> bool {
        if self.dirty == false {
            let mut state = self.state.borrow_mut();
            if state.lock(&self.row.key) == false {
                eprintln!("Detected concurrent writes on data object ({:?}) - the last one in scope will override the all other changes made", self.row.key);
            }
            self.dirty = true;
        }
        true
    }

    pub fn flush(&mut self) -> std::result::Result<(), SerializationError> {
        if self.dirty == true
        {            
            let mut state = self.state.borrow_mut();
            state.unlock(&self.row.key);
            
            self.dirty = false;

            let row_data = self.row.as_row_data()?;
            state.dirty(&self.row.key, row_data);
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), FlushError>
    {
        // If it has no authorizations then we can't write it anywhere
        if self.auth().allow_write.len() <= 0 {
            return Err(FlushError::LintError(LintError::NoAuthorization(self.row.key.clone())));
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
    pub fn delete(self) {
        let mut state = self.state.borrow_mut();
        if state.lock(&self.row.key) == false {
            eprintln!("Detected concurrent write while deleting a data object ({:?}) - the delete operation will override everything else", self.row.key);
        }
        let key = self.key().clone();
        state.cache_store.remove(&key);
        state.cache_load.remove(&key);
        state.deleted.insert(key, self.auth().clone());
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
        // Do some basic checks
        if let Err(err) = self.validate() {
            debug_assert!(false, "dao-validation-error {}", err.to_string());
        }

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
    cache_store: FxHashMap<PrimaryKey, Rc<RowData<M>>>,
    cache_load: FxHashMap<PrimaryKey, Rc<EventExt<M>>>,
    locked: FxHashSet<PrimaryKey>,
    deleted: FxHashMap<PrimaryKey, MetaAuthorization>,
}

impl<M> DioState<M>
where M: OtherMetadata,
{
    fn dirty(&mut self, key: &PrimaryKey, row: RowData<M>) {
        let row = Rc::new(row);
        self.store.push(row.clone());
        self.cache_store.insert(key.clone(), row);
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
            cache_store: FxHashMap::default(),
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
            meta: M::default(),
            data: data,
            auth: MetaAuthorization::default(),
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
        if let Some(dao) = state.cache_store.get(key) {
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

        let entry = match multi.lookup(key) {
            Some(a) => a,
            None => return Result::Err(LoadError::NotFound(key.clone()))
        };
        if entry.meta.get_tombstone().is_some() {
            return Result::Err(LoadError::Tombstoned(key.clone()));
        }

        let mut evt = multi.load(&entry)?;
        evt.raw.data = match evt.raw.data {
            Some(data) => Some(multi.data_as_overlay(&mut evt.raw.meta, data)?),
            None => None,
        };

        let row = Row::from_event(&evt)?;
        state.cache_load.insert(key.clone(), Rc::new(evt));
        Ok(DaoExt::new(row, &self.state))
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
                    
                    // Perform any transformation (e.g. data encryption)
                    let data = multi.data_as_underlay(&mut meta, row.data.clone())?;
                    let data_hash = super::crypto::Hash::from_bytes(&data[..]);

                    // Compute all the extra metadata for an event
                    let extra_meta = multi.metadata_lint_event(&Some(data_hash), &mut meta, &self.session)?;
                    meta.core.extend(extra_meta);
                    
                    let evt = EventRaw {
                        meta: meta,
                        data_hash: Some(data_hash),
                        data: Some(data),
                    }.as_plus()?;
                    evts.push(evt);
                }

                // Build events that will represent tombstones on all these records (they will be sent after the writes)
                for (key, auth) in &state.deleted {
                    let mut meta = MetadataExt::default();
                    meta.core.push(CoreMetadata::Authorization(auth.clone()));

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
pub enum TestDao
{
    Blah1,
    Blah2(u32),
    Blah3(String),
    Blah4,
    Blah5,
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

    session.properties.push(SessionProperty::WriteKey(write_key.clone()));
    session.properties.push(SessionProperty::WriteKey(write_key2.clone()));
    session.properties.push(SessionProperty::ReadKey(read_key.clone()));
    session.properties.push(SessionProperty::Identity("author@here.com".to_string()));

    let key1;
    let key2;

    {
        let mut dio = chain.dio(&session);

        // Write a value immediately from chain (this data will remain in the transaction)
        {
            {
                let mut dao1 = dio.store(TestDao::Blah1).unwrap();
                key1 = dao1.key().clone();
                println!("key1: {}", key1.as_hex_string());
                
                dio.load::<TestDao>(&key1).expect_err("This load is meant to fail as we are still editing the object");

                dao1.auth_mut().allow_write.push(write_key.hash());
            }

            dio.flush().unwrap();

            {
                // Load the object again which should load it from the cache
                let mut dao1: Dao<TestDao> = dio.load(&key1).unwrap();

                // When we update this value it will become dirty and hence should block future loads until its flushed or goes out of scope
                *dao1 = TestDao::Blah2(2);
                dio.load::<TestDao>(&key1).expect_err("This load is meant to fail due to a lock being triggered");

                // Flush the data and attempt to read it again (this should succeed)
                dao1.flush().expect("Flush failed");
                match *dio.load(&key1).expect("The dirty data object should have been read after it was flushed") {
                    TestDao::Blah2(a) => assert_eq!(a.clone(), 2 as u32),
                    _ => panic!("Data is not saved correctly")
                }
            }

            {
                // Load the object again which should load it from the cache
                let mut dao1: Dao<TestDao> = dio.load(&key1).unwrap();
            
                // Again after changing the data reads should fail
                *dao1 = TestDao::Blah3("testblah".to_string());
                dio.load::<TestDao>(&key1).expect_err("This load is meant to fail due to a lock being triggered");
            }

            {
                // Write a record to the chain that we will delete again later
                let mut dao2 = dio.store(TestDao::Blah4).unwrap();
                
                // We create a new private key for this data
                dao2.auth_mut().allow_write.push(write_key.as_public_key().hash());
                //dao2.auth_mut().allow_write.push(write_key2.as_public_key().hash());
                
                key2 = dao2.key().clone();
                println!("key2: {}", key2.as_hex_string());
            }
        }

        // Now its out of scope it should be loadable again
        match &*dio.load(&key1).expect("The dirty data object should have been read after it was flushed") {
            TestDao::Blah3(a) => assert_eq!(a.clone(), "testblah".to_string()),
            _ => panic!("Data is not saved correctly")
        }
    }

    {
        let mut dio = chain.dio(&session);

        // The data we saved earlier should be accessible accross DIO scope boundaries
        let mut dao1 = dio.load(&key1).expect("The data object should have been read");
        match &*dao1 {
            TestDao::Blah3(a) => assert_eq!(a.clone(), "testblah".to_string()),
            _ => panic!("Data is not saved correctly")
        }
        *dao1 = TestDao::Blah4;

        // First attempt to read the record then delete it
        let dao2 = dio.load::<TestDao>(&key2).expect("The record should load before we delete it in this session");
        dao2.delete();

        // It should no longer load now that we deleted it
        dio.load::<TestDao>(&key2).expect_err("This load should fail as we deleted the record");
    }

    {
        let mut dio = chain.dio(&session);

        // After going out of scope then back again we should still no longer see the record we deleted
        dio.load::<TestDao>(&key2).expect_err("This load should fail as we deleted the record");
    }

    //chain.single().destroy().unwrap();
}