use std::rc::Rc;

#[allow(unused_imports)]
use super::meta::*;
#[allow(unused_imports)]
use fastrand::u64;

pub use ate_crypto::spec::PrimaryKey;

pub(crate) struct PrimaryKeyScope {
    pop: Option<PrimaryKey>,
    _negative: Rc<()>,
}

impl PrimaryKeyScope {
    pub fn new(key: PrimaryKey) -> Self {
        PrimaryKeyScope {
            pop: PrimaryKey::current_set(Some(key)),
            _negative: Rc::new(()),
        }
    }
}

impl Drop for PrimaryKeyScope {
    fn drop(&mut self) {
        PrimaryKey::current_set(self.pop.take());
    }
}

impl Metadata {
    pub fn for_data(key: PrimaryKey) -> Metadata {
        let mut ret = Metadata::default();
        ret.core.push(CoreMetadata::Data(key));
        return ret;
    }

    pub fn get_data_key(&self) -> Option<PrimaryKey> {
        self.core
            .iter()
            .filter_map(|m| match m {
                CoreMetadata::Data(k) => Some(k.clone()),
                CoreMetadata::Tombstone(k) => Some(k.clone()),
                _ => None,
            })
            .next()
    }

    #[allow(dead_code)]
    pub fn set_data_key(&mut self, key: PrimaryKey) {
        for core in self.core.iter_mut() {
            match core {
                CoreMetadata::Data(k) => {
                    if *k == key {
                        return;
                    }
                    *k = key;
                    return;
                }
                _ => {}
            }
        }
        self.core.push(CoreMetadata::Data(key));
    }
}