use std::marker::PhantomData;
use fxhash::FxHashMap;

use serde::*;
use serde::de::*;
use crate::meta::*;
use crate::dio::*;
use crate::dio::dao::*;
use crate::error::*;
use std::collections::VecDeque;

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

    pub fn get(&self, chain_url: url::Url) -> Option<PrimaryKey>
    {
        let key = chain_url.to_string();
        match self.map.get(&key) {
            Some(a) => Some(a.clone()),
            None => None
        }
    }

    pub fn set(&mut self, chain_url: url::Url, key: PrimaryKey)
    {
        let chain_key = chain_url.to_string();
        self.map.insert(chain_key, key);
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