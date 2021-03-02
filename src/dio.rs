use linked_hash_map::LinkedHashMap;
use fxhash::{FxHashMap, FxHashSet};

#[cfg(test)]
use serde::{Deserialize};
use serde::{Serialize, de::DeserializeOwned};
use tokio::io::Result;
use tokio::io::Error;
use tokio::io::ErrorKind;
use bytes::Bytes;
use std::cell::{RefCell};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use super::header::*;
use super::chain::*;
use super::event::*;

#[allow(dead_code)]
type Dio<'a, D> = DioExt<'a, EmptyMetadata, D>;
#[allow(dead_code)]
type Dao<D> = DaoExt<EmptyMetadata, D>;

#[derive(Debug, Clone)]
pub struct Row<M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone
{
    pub key: PrimaryKey,
    pub meta: Metadata<M>,
    pub data: D,
}

impl<M, D> Row<M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone
{
    pub fn new(key: PrimaryKey, meta: Metadata<M>, data: D) -> Row<M, D>
    {
        Row {
            key: key,
            meta: meta,
            data: data
        }
    }
}

#[derive(Debug)]
pub struct DaoExt<M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
{
    hard_copy: Option<Row<M, D>>,
    soft_copy: Rc<Row<M, D>>,
    state: Rc<RefCell<DioState<M, D>>>
}

impl<M, D> DaoExt<M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
{
    fn new<>(what: &Rc<Row<M, D>>, state: &Rc<RefCell<DioState<M, D>>>) -> DaoExt<M, D> {
        DaoExt {
            hard_copy: None,
            soft_copy: Rc::clone(what),
            state: Rc::clone(state),
        }
    }

    fn fork(&mut self) -> bool {
        if self.hard_copy.is_none() {
            let mut state = self.state.borrow_mut();
            if state.lock(&self.soft_copy.key) == false {
                eprintln!("Detected concurrent writes on data object ({:?}) - the last one in scope will override the all other changes made", self.soft_copy.key);
            }
            self.hard_copy = Some(self.soft_copy.deref().clone());
        }
        true
    }

    pub fn flush(&mut self) {
        if let Some(dao) = self.hard_copy.take() {
            let mut state = self.state.borrow_mut();
            state.unlock(&dao.key);
            state.dirty(dao);
        }
    }

    #[allow(dead_code)]
    pub fn key(&self) -> &PrimaryKey {
        &self.soft_copy.key
    }

    #[allow(dead_code)]
    pub fn metadata(&self) -> &Metadata<M> {
        &self.soft_copy.meta
    }

    #[allow(dead_code)]
    pub fn metadata_mut(&mut self) -> &mut Metadata<M> {
        self.fork();
        &mut self.hard_copy
            .as_mut()
            .expect("Something strange happened, we just set a variable then tried to read it, major bug!")
            .meta
    }

    #[allow(dead_code)]
    pub fn delete(self) {
        let mut state = self.state.borrow_mut();
        if state.lock(&self.soft_copy.key) == false {
            eprintln!("Detected concurrent write while deleting a data object ({:?}) - the delete operation will override everything else", self.soft_copy.key);
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
        match self.hard_copy.as_ref() {
            Some(a) => &a.data,
            None => &self.soft_copy.deref().data,
        }
    }
}

impl<M, D> DerefMut for DaoExt<M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.fork();
        &mut self.hard_copy
            .as_mut()
            .expect("Something strange happened, we just set a variable then tried to read it, major bug!")
            .data
    }
}

impl<M, D> Drop for DaoExt<M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
{
    fn drop(&mut self) {
        self.flush();
    }
}

#[derive(Debug)]
struct DioState<M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
{
    dirty: LinkedHashMap<PrimaryKey, Rc<Row<M, D>>>,
    cache: FxHashMap<PrimaryKey, Rc<Row<M, D>>>,
    locked: FxHashSet<PrimaryKey>,
    deleted: FxHashSet<PrimaryKey>,
}

impl<M, D> DioState<M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
{
    fn dirty(&mut self, dao: Row<M, D>) {
        self.cache.remove(&dao.key);
        self.dirty.insert(dao.key, Rc::new(dao));
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

impl<M, D> DioState<M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
{
    fn new() -> DioState<M, D> {
        DioState {
            dirty: LinkedHashMap::new(),
            cache: FxHashMap::default(),
            locked: FxHashSet::default(),
            deleted: FxHashSet::default(),
        }
    }
}

pub struct DioExt<'a, M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
{
    accessor: ChainAccessorExt<M>,
    multi: Option<ChainMultiUserExt<'a, M>>,
    state: Rc<RefCell<DioState<M, D>>>,
}

impl<'a, M, D> DioExt<'a, M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
{
    #[allow(dead_code)]
    pub fn store(&mut self, metadata: Metadata<M>, data: D) -> Result<DaoExt<M, D>> {
        let key = PrimaryKey::generate();
        let dao = Rc::new(Row::new(key.clone(), metadata, data));
        self.state.borrow_mut().dirty.insert(key, dao.clone());
        Ok(DaoExt::new(&dao, &self.state))
    }

    #[allow(dead_code)]
    pub fn load(&mut self, key: &PrimaryKey) -> Result<DaoExt<M, D>> {
        let mut state = self.state.borrow_mut();
        if state.is_locked(key) {
            return Result::Err(Error::new(ErrorKind::Other, format!("The record is locked as it has a dirty record already in scope for this call stack {:?}", key)));
        }
        if let Some(dao) = state.dirty.get(key) {
            return Ok(DaoExt::new(dao, &self.state));
        }
        if let Some(dao) = state.cache.get(key) {
            return Ok(DaoExt::new(dao, &self.state));
        }
        if state.deleted.contains(key) {
            return Result::Err(Error::new(ErrorKind::NotFound, format!("Record with this key has already been deleted {}", key.as_hex_string())))
        }

        let multi = self.multi.as_ref().ok_or(Error::new(ErrorKind::Other, "Dio is not properly initialized (missing multiuser chain handle)"))?;

        let entry = match multi.search(key) {
            Some(a) => a,
            None => return Result::Err(Error::new(ErrorKind::NotFound, format!("Failed to find a record for {}", key.as_hex_string()))),
        };
        if entry.header.meta.has_tombstone() {
            return Result::Err(Error::new(ErrorKind::NotFound, format!("Found a record but its been tombstoned for {}", key.as_hex_string())));
        }

        let mut evt = multi.load(&entry)?;

        match evt.body {
            Some(data) => {
                let transformed_data = multi.data_as_overlay(&mut evt.header.meta, data)?;
                match bincode::deserialize(&transformed_data) {
                    std::result::Result::Ok(a) => {
                        let dao = Rc::new(Row::new(key.clone(), evt.header.meta, a));
                        state.cache.insert(key.clone(), dao.clone());
                        Ok(DaoExt::new(&dao, &self.state))
                    },
                    std::result::Result::Err(err) => Result::Err(Error::new(ErrorKind::Other, format!("{}", err))),
                }
            }
            None => return Result::Err(Error::new(ErrorKind::NotFound, format!("Found a record but it has no data to load for {}", key.as_hex_string()))),
        }
    }

    #[allow(dead_code)]
    pub fn erase(&mut self, key: &PrimaryKey) -> Result<()> {
        let dao = self.load(key)?;
        dao.delete();
        Ok(())
    }
}

impl<'a, M, D> Drop
for DioExt<'a, M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
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
                for (_, dao) in &state.dirty {
                    let dao = dao.deref();
                    let mut meta = Metadata::default();

                    let linted_data = Bytes::from(bincode::serialize(&dao.data).unwrap());
                    let data = match multi.data_as_underlay(&mut meta, linted_data) {
                        Ok(a) => a,
                        Err(err) => {
                            debug_assert!(false, err.to_string());
                            eprintln!("{}", err.to_string());
                            continue;
                        },
                    };

                    let evt = Event {
                        header: Header {
                            key: dao.key,
                            meta: meta,
                        },
                        body: Some(data),
                    };
                    evts.push(evt);
                }
            }

            // Build events that will represent tombstones on all these records (they will be sent after the writes)
            for key in &state.deleted {
                let mut evt = Event {
                    header: Header {
                        key: key.clone(),
                        meta: Metadata::default(),
                    },
                    body: None,
                };
                evt.header.meta.add_tombstone();
                evts.push(evt);
            }

            // Process it in the chain of trust
            let mut single = self.accessor.single();
            single.feed(evts).unwrap();
        }
    }
}

impl<M> Metadata<M>
where M: OtherMetadata
{
    #[allow(dead_code)]
    pub fn add_tombstone(&mut self) {
        if self.has_tombstone() == true { return; }
        self.core.push(CoreMetadata::Tombstone);
    }
}

pub trait DioFactoryExt<'a, M, D>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
{
    fn dio(&'a mut self) -> DioExt<'a, M, D>;
}

impl<'a, M, D> DioFactoryExt<'a, M, D>
for ChainAccessorExt<M>
where M: OtherMetadata,
      D: Serialize + DeserializeOwned + Clone,
{
    fn dio(&'a mut self) -> DioExt<'a, M, D> {
        let accessor = ChainAccessorExt::from_accessor(self); 
        let multi = self.multi();
        DioExt {
            accessor: accessor,
            state: Rc::new(RefCell::new(DioState::new())),
            multi: Some(multi),          
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
    let mut chain = create_test_chain("test_dio".to_string());

    let key1;
    let key2;
    {
        let mut dio = chain.dio();

        // Write a value immediately from chain (this data will remain in the transaction)
        {
            let mut dao1 = dio.store(Metadata::default(), TestDao::Blah1).unwrap();
            key1 = dao1.key().clone();

            // Attempting to load the data object while it is still in scope and not flushed will fail
            match dio.load(&key1).expect("The data object should have been read").deref() {
                TestDao::Blah1 => {},
                _ => panic!("Data is not saved correctly")
            }

            // When we update this value it will become dirty and hence should block future loads until its flushed or goes out of scope
            *dao1 = TestDao::Blah2(2);
            dio.load(&key1).expect_err("This load is meant to fail due to a lock being triggered");

            // Flush the data and attempt to read it again (this should succeed)
            dao1.flush();
            match dio.load(&key1).expect("The dirty data object should have been read after it was flushed").deref() {
                TestDao::Blah2(a) => assert_eq!(a.clone(), 2 as u32),
                _ => panic!("Data is not saved correctly")
            }

            // Again after changing the data reads should fail
            *dao1 = TestDao::Blah3("testblah".to_string());
            dio.load(&key1).expect_err("This load is meant to fail due to a lock being triggered");

            // Write a record to the chain that we will delete again later
            let dao2 = dio.store(Metadata::default(), TestDao::Blah4).unwrap();
            key2 = dao2.key().clone();
        }

        // Now its out of scope it should be loadable again
        match dio.load(&key1).expect("The dirty data object should have been read after it was flushed").deref() {
            TestDao::Blah3(a) => assert_eq!(a.clone(), "testblah".to_string()),
            _ => panic!("Data is not saved correctly")
        }
    }

    {
        let mut dio = chain.dio();

        // First attempt to read the record then delete it
        let dao2: Dao<TestDao> = dio.load(&key2).expect("The record should load before we delete it in this session");
        dao2.delete();

        // It should no longer load now that we deleted it
        dio.load(&key2).expect_err("This load should fail as we deleted the record");
    }

    {
        let mut dio: Dio<TestDao> = chain.dio();

        // After going out of scope then back again we should still no longer see the record we deleted
        dio.load(&key2).expect_err("This load should fail as we deleted the record");
    }

    //chain.single().destroy().unwrap();
}