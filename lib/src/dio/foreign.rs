use fxhash::FxHashMap;

use serde::*;
use crate::header::*;

/// Rerepresents a reference to structured data that exists in another
/// chain-of-trust
///
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DaoForeign
{
    map: FxHashMap<String, PrimaryKey>,
}

impl DaoForeign
{
    pub fn new() -> DaoForeign {
        DaoForeign {
            map: FxHashMap::default(),
        }
    }

    pub fn get_by_url(&self, chain_url: url::Url) -> Option<PrimaryKey>
    {
        let key = chain_url.to_string();
        match self.map.get(&key) {
            Some(a) => Some(a.clone()),
            None => None
        }
    }

    pub fn set_by_url(&mut self, chain_url: url::Url, key: PrimaryKey)
    {
        let chain_key = chain_url.to_string();
        self.map.insert(chain_key, key);
    }

    pub fn get_by_name(&self, name: String) -> Option<PrimaryKey>
    {
        match self.map.get(&name) {
            Some(a) => Some(a.clone()),
            None => None
        }
    }

    pub fn set_by_name(&mut self, name: String, key: PrimaryKey)
    {
        self.map.insert(name, key);
    }

    pub fn get<T>(&self) -> Option<PrimaryKey>
    {
        let name = std::any::type_name::<T>().to_string();
        match self.map.get(&name) {
            Some(a) => Some(a.clone()),
            None => None
        }
    }

    pub fn set<T>(&mut self, key: PrimaryKey)
    {
        let name = std::any::type_name::<T>().to_string();
        self.map.insert(name, key);
    }
}

impl Default
for DaoForeign
{
    fn default() -> DaoForeign
    {
        DaoForeign::new()
    }
}