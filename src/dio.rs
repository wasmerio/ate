use linked_hash_map::LinkedHashMap;
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
}

impl<D, M> Row<M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
{
    #[allow(dead_code)]
    pub fn new(key: PrimaryKey, meta: M, data: D) -> Row<M, D> {
        Row {
            key: key,
            meta: meta,
            data: data,
        }
    }

    pub fn from_event(evt: &EventExt<M>) -> Result<Row<M, D>, EventSerializationError> {
        let key = match evt.raw.meta.get_data_key() {
            Some(key) => key,
            None => { return Result::Err(EventSerializationError::NoPrimarykey) }
        };
        match &evt.raw.data {
            Some(data) => {
                Ok(
                    Row {
                        key: key,
                        meta: evt.raw.meta.other.clone(),
                        data: bincode::deserialize(&data)?,
                    }
                )
            }
            None => return Result::Err(EventSerializationError::NoData),
        }
    }

    pub fn from_row_data(row: &RowData<M>) -> Result<Row<M, D>, EventSerializationError> {
        Ok(
            Row {
                key: row.key,
                meta: row.meta.clone(),
                data: bincode::deserialize(&row.data)?,
            }
        )
    }

    pub fn as_row_data(&self) -> std::result::Result<RowData<M>, EventSerializationError> {
        let data = Bytes::from(bincode::serialize(&self.data)?);
        let data_hash = super::crypto::Hash::from_bytes(&data[..]);
        Ok
        (
            RowData {
                key: self.key.clone(),
                meta: self.meta.clone(),
                data_hash: data_hash,
                data: data,
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
}

#[derive(Debug)]
pub struct DaoExt<M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
{
    dirty: bool,
    row: Row<M, D>,
    state: Rc<RefCell<DioState<M>>>
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

    pub fn flush(&mut self) -> std::result::Result<(), EventSerializationError> {
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
    pub fn delete(self) {
        let mut state = self.state.borrow_mut();
        if state.lock(&self.row.key) == false {
            eprintln!("Detected concurrent write while deleting a data object ({:?}) - the delete operation will override everything else", self.row.key);
        }
        let key = self.key().clone();
        state.dirty.remove(&key);
        state.cache.remove(&key);
        state.deleted.insert(key);
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
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

#[derive(Debug)]
struct DioState<M>
where M: OtherMetadata,
{
    dirty: LinkedHashMap<PrimaryKey, Rc<RowData<M>>>,
    cache: FxHashMap<PrimaryKey, Rc<EventExt<M>>>,
    locked: FxHashSet<PrimaryKey>,
    deleted: FxHashSet<PrimaryKey>,
}

impl<M> DioState<M>
where M: OtherMetadata,
{
    fn dirty(&mut self, key: &PrimaryKey, row: RowData<M>) {
        self.cache.remove(key);
        self.dirty.insert(key.clone(), Rc::new(row));
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
            dirty: LinkedHashMap::new(),
            cache: FxHashMap::default(),
            locked: FxHashSet::default(),
            deleted: FxHashSet::default(),
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
    pub fn store<D>(&mut self, data: D) -> DaoExt<M, D>
    where D: Serialize + DeserializeOwned + Clone,
    {
        let key = PrimaryKey::generate();
        let body = Bytes::from(bincode::serialize(&data).unwrap());
        let body_hash = super::crypto::Hash::from_bytes(&body[..]);
        let meta = M::default();

        let row = RowData {
            key: key,
            meta: meta.clone(),
            data_hash: body_hash,
            data: body,
        };

        self.state.borrow_mut().dirty.insert(key.clone(), Rc::new(row));

        let row = Row {
            key: key,
            meta: meta,
            data: data,
        };
        DaoExt::new(row, &self.state)
    }

    #[allow(dead_code)]
    pub fn load<D>(&mut self, key: &PrimaryKey) -> Result<DaoExt<M, D>, LoadError>
    where D: Serialize + DeserializeOwned + Clone,
    {
        let mut state = self.state.borrow_mut();
        if state.is_locked(key) {
            return Result::Err(LoadError::Locked);
        }
        if let Some(dao) = state.dirty.get(key) {
            let row = Row::from_row_data(dao.deref())?;
            return Ok(DaoExt::new(row, &self.state));
        }
        if let Some(dao) = state.cache.get(key) {
            let row = Row::from_event(dao.deref())?;
            return Ok(DaoExt::new(row, &self.state));
        }
        if state.deleted.contains(key) {
            return Result::Err(LoadError::AlreadyDeleted);
        }

        let multi = self.multi.as_ref().ok_or(LoadError::InternalError("Dio is not properly initialized (missing multiuser chain handle)".to_string()))?;

        let entry = match multi.lookup(key) {
            Some(a) => a,
            None => return Result::Err(LoadError::NotFound)
        };
        if entry.meta.get_tombstone().is_some() {
            return Result::Err(LoadError::Tombstoned);
        }

        let mut evt = multi.load(&entry)?;
        evt.raw.data = match evt.raw.data {
            Some(data) => Some(multi.data_as_overlay(&mut evt.raw.meta, data)?),
            None => None,
        };

        let row = Row::from_event(&evt)?;
        state.cache.insert(key.clone(), Rc::new(evt));
        Ok(DaoExt::new(row, &self.state))
    }
}

impl<'a, M> Drop
for DioExt<'a, M>
where M: OtherMetadata,
{
    fn drop(&mut self)
    {
        // If we have dirty records
        let state = self.state.borrow_mut();
        if state.dirty.is_empty() == false || state.deleted.is_empty() == false
        {
            let mut evts = Vec::new();
            {
                // Take the reference to the multi for a limited amount of time then destruct it and release the lock
                let multi = self.multi.take().expect("The multilock was released before the drop call was triggered by the Dio going out of scope.");

                // Convert all the events that we are storing into serialize data
                let mut data_hashes = Vec::new();
                for (_, dao) in state.dirty.iter() {
                    let mut meta = MetadataExt::for_data(dao.key);
                    meta.other = dao.meta.clone();
                    
                    let data = match multi.data_as_underlay(&mut meta, dao.data.clone()) {
                        Ok(a) => a,
                        Err(err) => {
                            debug_assert!(false, err.to_string());
                            eprintln!("{}", err.to_string());
                            continue;
                        },
                    };
                    let data_hash = super::crypto::Hash::from_bytes(&data[..]);
                    data_hashes.push(data_hash);

                    multi.metadata_lint_event(&Some(data_hash), &mut meta, &self.session);
                    
                    let evt = EventRaw {
                        meta: meta,
                        data_hash: Some(data_hash),
                        data: Some(data),
                    };
                    evts.push(evt);
                }

                // Lint the data
                let meta = match multi.metadata_lint_many(&data_hashes, &self.session) {
                    Ok(a) => a,
                    Err(err) => {
                        debug_assert!(false, err.to_string());
                        eprintln!("{}", err.to_string());
                        Vec::new()
                    },
                };

                // If it has data then insert it at the front of these events
                if meta.len() > 0 {
                    evts.insert(0, EventRaw {
                        meta: MetadataExt {
                            core: meta,
                            other: M::default(),
                        },
                        data_hash: None,
                        data: None,
                    });
                }
            }

            // Build events that will represent tombstones on all these records (they will be sent after the writes)
            for key in &state.deleted {
                let mut meta = MetadataExt::default();
                meta.add_tombstone(key.clone());
                let evt = EventRaw {
                    meta: meta,
                    data_hash: None,
                    data: None,
                };
                evts.push(evt);
            }

            // Process it in the chain of trust
            let mut single = self.accessor.single();
            single.event_feed(evts).unwrap();
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
}

#[test]
fn test_dio()
{
    let mut session = Session::default();
    let mut chain = create_test_chain("test_dio".to_string(), true);

    let write_key = EncryptKey::generate(crate::crypto::KeySize::Bit192);
    let read_key = EncryptKey::generate(crate::crypto::KeySize::Bit192);
    session.properties.push(SessionProperty::WriteKey(write_key));
    session.properties.push(SessionProperty::ReadKey(read_key));
    session.properties.push(SessionProperty::Identity("author@here.com".to_string()));

    let key1;
    let key2;

    {
        let mut dio = chain.dio(&session);

        // Write a value immediately from chain (this data will remain in the transaction)
        {
            let mut dao1 = dio.store(TestDao::Blah1);
            key1 = dao1.key().clone();

            // Attempting to load the data object while it is still in scope and not flushed will fail
            match *dio.load(&key1).expect("The data object should have been read") {
                TestDao::Blah1 => {},
                _ => panic!("Data is not saved correctly")
            }

            // When we update this value it will become dirty and hence should block future loads until its flushed or goes out of scope
            *dao1 = TestDao::Blah2(2);
            dio.load::<TestDao>(&key1).expect_err("This load is meant to fail due to a lock being triggered");

            // Flush the data and attempt to read it again (this should succeed)
            dao1.flush().expect("Flush failed");
            match *dio.load(&key1).expect("The dirty data object should have been read after it was flushed") {
                TestDao::Blah2(a) => assert_eq!(a.clone(), 2 as u32),
                _ => panic!("Data is not saved correctly")
            }

            // Again after changing the data reads should fail
            *dao1 = TestDao::Blah3("testblah".to_string());
            dio.load::<TestDao>(&key1).expect_err("This load is meant to fail due to a lock being triggered");

            // Write a record to the chain that we will delete again later
            let dao2 = dio.store(TestDao::Blah4);
            key2 = dao2.key().clone();
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
        match &*dio.load(&key1).expect("The data object should have been read") {
            TestDao::Blah3(a) => assert_eq!(a.clone(), "testblah".to_string()),
            _ => panic!("Data is not saved correctly")
        }

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