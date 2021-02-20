use serde::{Serialize, de::DeserializeOwned};

#[cfg(test)]
use tokio::runtime::Runtime;
use tokio::io::Error;
use tokio::io::ErrorKind;
use tokio::io::Result;

use crate::{redo::EventData};

#[allow(unused_imports)]
use super::conf::*;
#[allow(unused_imports)]
use super::header::*;
use super::validator::*;

#[allow(unused_imports)]
use std::io::Write;
use super::redo::RedoLog;
use super::conf::ConfigStorage;


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
    where M: Serialize + DeserializeOwned + Clone
{
    key: ChainKey,
    redo: RedoLog,
    events: Vec<Event<M>>,
}

impl<M> ChainOfTrust<M>
    where M: Serialize + DeserializeOwned + Clone
{
    #[allow(dead_code)]
    pub async fn new(cfg: &impl ConfigStorage, key: &ChainKey) -> Result<ChainOfTrust<M>> {
        Ok(
            ChainOfTrust {
                key: key.clone(),
                redo: RedoLog::new(cfg, key).await?,
                events: Vec::new(),
            }
        )
    }

    #[allow(dead_code)]
    pub async fn process(&mut self, evt_data: EventData, validator: impl EventValidator) -> Result<()> {
        let evt: Option<Event<M>> = Event::from_event_data(&evt_data);
        match evt {
            Some(evt) => {
                validator.validate(&evt, &self.events)?;
                self.redo.write(evt_data).await?;
                Ok(())
            },
            _ => {
                Result::Err(
                    Error::new(ErrorKind::Other, "Failed to deserialize the event data")
                )
            }
        }
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