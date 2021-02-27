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
use super::validator::*;
use super::index::*;
use super::compact::*;
use super::event::*;

#[derive(Debug, Clone)]
pub struct Dao<M, D>
where M: MetadataTrait,
      D: Serialize + DeserializeOwned + Clone
{
    pub key: PrimaryKey,
    pub meta: M,
    pub data: D,
}

impl<M, D> Dao<M, D>
where M: MetadataTrait,
      D: Serialize + DeserializeOwned + Clone
{
    pub fn new(key: PrimaryKey, meta: M, data: D) -> Dao<M, D>
    {
        Dao {
            key: key,
            meta: meta,
            data: data
        }
    }
}

#[derive(Debug)]
pub struct DaoRef<M, D>
where M: MetadataTrait,
      D: Serialize + DeserializeOwned + Clone,
{
    hard_copy: Option<Dao<M, D>>,
    soft_copy: Rc<Dao<M, D>>,
    state: Rc<RefCell<DioState<M, D>>>
}

impl<M, D> DaoRef<M, D>
where M: MetadataTrait,
      D: Serialize + DeserializeOwned + Clone,
{
    fn new<>(what: &Rc<Dao<M, D>>, state: &Rc<RefCell<DioState<M, D>>>) -> DaoRef<M, D> {
        DaoRef {
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
    pub fn metadata(&self) -> &M {
        &self.soft_copy.meta
    }

    #[allow(dead_code)]
    pub fn metadata_mut(&mut self) -> &mut M {
        self.fork();
        &mut self.hard_copy
            .as_mut()
            .expect("Something strange happened, we just set a variable then tried to read it, major bug!")
            .meta
    }
}

impl<M, D> Deref for DaoRef<M, D>
where M: MetadataTrait,
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

impl<M, D> DerefMut for DaoRef<M, D>
where M: MetadataTrait,
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

impl<M, D> Drop for DaoRef<M, D>
where M: MetadataTrait,
      D: Serialize + DeserializeOwned + Clone,
{
    fn drop(&mut self) {
        self.flush();
    }
}

#[derive(Debug)]
struct DioState<M, D>
where M: MetadataTrait,
      D: Serialize + DeserializeOwned + Clone,
{
    dirty: LinkedHashMap<PrimaryKey, Rc<Dao<M, D>>>,
    cache: FxHashMap<PrimaryKey, Rc<Dao<M, D>>>,
    locked: FxHashSet<PrimaryKey>,
}

impl<M, D> DioState<M, D>
where M: MetadataTrait,
      D: Serialize + DeserializeOwned + Clone,
{
    fn dirty(&mut self, dao: Dao<M, D>) {
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
where M: MetadataTrait,
      D: Serialize + DeserializeOwned + Clone,
{
    fn new() -> DioState<M, D> {
        DioState {
            dirty: LinkedHashMap::new(),
            cache: FxHashMap::default(),
            locked: FxHashSet::default(),
        }
    }
}

pub struct Dio<'a, M, D, I, V, C>
where M: MetadataTrait,
      D: Serialize + DeserializeOwned + Clone,
      I: EventIndexer<M>,
      V: EventValidator<M, Index=I>,
      C: EventCompactor<M, Index=I>,
{
    accessor: ChainAccessor<M, I, V, C>,
    multi: Option<ChainMultiUser<'a, M, I, V, C>>,
    state: Rc<RefCell<DioState<M, D>>>,
}

impl<'a, M, D, I, V, C> Dio<'a, M, D, I, V, C>
where M: MetadataTrait,
      D: Serialize + DeserializeOwned + Clone,
      I: EventIndexer<M>,
      V: EventValidator<M, Index=I>,
      C: EventCompactor<M, Index=I>,
{
    #[allow(dead_code)]
    pub fn create(&mut self, metadata: M, data: D) -> Result<DaoRef<M, D>> {
        let key = PrimaryKey::generate();
        let dao = Rc::new(Dao::new(key.clone(), metadata, data));
        self.state.borrow_mut().dirty.insert(key, dao.clone());
        Ok(DaoRef::new(&dao, &self.state))
    }

    #[allow(dead_code)]
    pub fn load(&mut self, key: &PrimaryKey) -> Result<DaoRef<M, D>> {
        let state = self.state.borrow_mut();
        if state.is_locked(key) {
            return Result::Err(Error::new(ErrorKind::Other, format!("The record is locked as it has a dirty record already in scope for this call stack {:?}", key)));
        }
        if let Some(dao) = state.dirty.get(key) {
            return Ok(DaoRef::new(dao, &self.state));
        }
        if let Some(dao) = state.cache.get(key) {
            return Ok(DaoRef::new(dao, &self.state));
        }

        let multi = self.multi.as_ref().ok_or(Error::new(ErrorKind::Other, "Dio is not properly initialized (missing multiuser chain handle)"))?;

        let entry = match multi.search(key) {
            Some(a) => a,
            None => return Result::Err(Error::new(ErrorKind::Other, format!("Failed to find a record for {:?}", key))),
        };

        let evt = multi.load(&entry)?;

        match bincode::deserialize(&evt.body) {
            std::result::Result::Ok(a) => {
                let dao = Rc::new(Dao::new(key.clone(), evt.header.meta, a));
                self.state.borrow_mut().cache.insert(key.clone(), dao.clone());
                Ok(DaoRef::new(&dao, &self.state))
            },
            std::result::Result::Err(err) => Result::Err(Error::new(ErrorKind::Other, format!("{}", err))),
        }
    }

    #[allow(dead_code)]
    #[allow(unused_variables)]
    pub fn delete(&'a mut self, key: &PrimaryKey) -> Result<bool> {
        Result::Err(Error::new(ErrorKind::Other, "Not implemented yet"))
    }
}

impl<'a, M, D, I, V, C> Drop
for Dio<'a, M, D, I, V, C>
where M: MetadataTrait,
      D: Serialize + DeserializeOwned + Clone,
      I: EventIndexer<M>,
      V: EventValidator<M, Index=I>,
      C: EventCompactor<M, Index=I>,
{
    fn drop(&mut self)
    {
        // We need to release the multi-user mode lock
        self.multi = None;        

        // If we have dirty records
        let state = self.state.borrow_mut();
        if state.dirty.is_empty() == false
        {
            // Convert all the events into serialize data
            let mut evts = Vec::new();
            for (_, dao) in &state.dirty {
                let dao = dao.deref();
                let evt = Event {
                    header: Header {
                        key: dao.key,
                        meta: dao.meta.clone(),
                    },
                    body: Bytes::from(bincode::serialize(&dao.data).unwrap()),
                };
                evts.push(evt);
            }

            // Process it in the chain of trust
            let mut single = self.accessor.single();
            single.process(evts).unwrap();
        }
    }
}

pub trait DioFactory<'a, M, D, I, V, C>
where M: MetadataTrait,
      D: Serialize + DeserializeOwned + Clone,
      I: EventIndexer<M>,
      V: EventValidator<M, Index=I>,
      C: EventCompactor<M, Index=I>,
{
    fn dio(&'a mut self) -> Dio<'a, M, D, I, V, C>;
}

impl<'a, M, D, I, V, C> DioFactory<'a, M, D, I, V, C>
for ChainAccessor<M, I, V, C>
where M: MetadataTrait,
      D: Serialize + DeserializeOwned + Clone,
      I: EventIndexer<M>,
      V: EventValidator<M, Index=I>,
      C: EventCompactor<M, Index=I>,
{
    fn dio(&'a mut self) -> Dio<'a, M, D, I, V, C> {
        let accessor = ChainAccessor::from_accessor(self); 
        let multi = self.multi();
        Dio {
            accessor: accessor,
            //gc: Rc::new(Arena::new()),
            state: Rc::new(RefCell::new(DioState::new())),
            multi: Some(multi),
            //cache: FxHashMap::default(),            
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
}

#[test]
fn test_dio()
{
    let mut chain = create_test_chain("test_dio".to_string());
    {
        let mut dio = chain.dio();

        // Write a value immediately from chain (this data will remain in the transaction)
        let key;
        {
            let mut dao = dio.create(DefaultMeta::default(), TestDao::Blah1).unwrap();
            key = dao.key().clone();

            // Attempting to load the data object while it is still in scope and not flushed will fail
            match dio.load(&key).expect("The data object should have been read").deref() {
                TestDao::Blah1 => {},
                _ => panic!("Data is not saved correctly")
            }

            // When we update this value it will become dirty and hence should block future loads until its flushed or goes out of scope
            *dao = TestDao::Blah2(2);
            dio.load(&key).expect_err("This load is meant to fail due to a lock being triggered");

            // Flush the data and attempt to read it again (this should succeed)
            dao.flush();
            match dio.load(&key).expect("The dirty data object should have been read after it was flushed").deref() {
                TestDao::Blah2(a) => assert_eq!(a.clone(), 2 as u32),
                _ => panic!("Data is not saved correctly")
            }

            // Again after changing the data reads should fail
            *dao = TestDao::Blah3("testblah".to_string());
            dio.load(&key).expect_err("This load is meant to fail due to a lock being triggered");
        }

        // Now its out of scope it should be loadable again
        match dio.load(&key).expect("The dirty data object should have been read after it was flushed").deref() {
            TestDao::Blah3(a) => assert_eq!(a.clone(), "testblah".to_string()),
            _ => panic!("Data is not saved correctly")
        }
    }

    chain.single().destroy().unwrap();
}