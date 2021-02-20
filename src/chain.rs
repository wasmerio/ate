use serde::{Serialize, Deserialize};

#[cfg(test)]
use tokio::runtime::Runtime;
#[allow(unused_imports)]
use super::conf::*;

#[allow(unused_imports)]
use std::io::Write;
use super::redo::RedoLog;
use super::conf::ConfigStorage;
use tokio::io::Result;

#[allow(unused_imports)]
use super::event::Event;
#[allow(unused_imports)]
use super::meta::*;

#[allow(dead_code)]
#[derive(Default, Clone)]
pub struct ChainKey {
    pub name: String,
}

impl ChainKey {
    #[allow(dead_code)]
    pub fn with_name(&self, val: &str) -> ChainKey
    {
        let mut ret = self.clone();
        ret.name = val.to_string();
        ret
    }
}

#[allow(dead_code)]
pub struct ChainOfTrust<M>
    where M: Serialize + Deserialize<'static> + Clone
{
    key: ChainKey,
    redo: RedoLog,
    events: Vec<Event<M>>,
}

impl<M> ChainOfTrust<M>
    where M: Serialize + Deserialize<'static> + Clone
{
    #[allow(dead_code)]
    pub async fn new(cfg: &impl ConfigStorage, key: &ChainKey) -> Result<ChainOfTrust<M>> {
        Ok(
            ChainOfTrust {
            key: key.clone(),
            redo: RedoLog::new(cfg, key).await?,
            events: Vec::new(),
        })
    }
}

#[test]
pub fn test_chain() {

    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let mock_cfg = mock_test_config();
        let mock_chain_key = ChainKey::default().with_name("test_chain");
        let _: ChainOfTrust<DefaultMeta> = ChainOfTrust::new(&mock_cfg, &mock_chain_key).await.expect("Failed to create the chain of trust");
    });
}