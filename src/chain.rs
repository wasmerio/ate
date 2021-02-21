use serde::{Serialize, de::DeserializeOwned};

#[cfg(test)]
use tokio::runtime::Runtime;
use tokio::io::Error;
use tokio::io::ErrorKind;
use tokio::io::Result;

#[allow(unused_imports)]
use super::conf::*;
#[allow(unused_imports)]
use super::header::*;
use super::validator::*;
use super::event::*;

#[allow(unused_imports)]
use std::io::Write;
use super::redo::RedoLog;
use super::conf::ConfigStorage;
#[allow(unused_imports)]
use bytes::Bytes;

#[allow(unused_imports)]
use super::event::Event;

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
pub struct ChainOfTrust
{
    key: ChainKey,
    redo: RedoLog,
}

impl ChainOfTrust
{
    #[allow(dead_code)]
    pub async fn new(cfg: &impl ConfigStorage, key: &ChainKey) -> Result<ChainOfTrust> {
        Ok(
            ChainOfTrust {
                key: key.clone(),
                redo: RedoLog::new(cfg, key).await?,
            }
        )
    }

    #[allow(dead_code)]
    pub async fn process<M: Serialize + DeserializeOwned + Clone>(&mut self, evt_data: EventData, validator: impl EventValidator) -> Result<Event<M>>
    {
        let evt: Option<Event<M>> = Event::from_event_data(&evt_data);
        match evt {
            Some(evt) => {
                validator.validate(&evt)?;
                self.redo.write(evt_data).await?;
                Ok(evt)
            },
            _ => {
                Result::Err(
                    Error::new(ErrorKind::Other, "Failed to deserialize the event data")
                )
            }
        }
    }

    /*
    pub async fn compact(&mut self) -> Result<()>
    {
        // first we need to trim any events that should no longer be there
        trim();

        // now we need to rebuild the redo log
        let flip = self.redo.begin_flip().await?;
        for evt in self.events {
            let evt = evt.to_event_data();
            let _ = flip.write(evt).await;
        }
        self.redo.end_flip(flip);

        Ok(())
    }*/
}

#[test]
pub fn test_chain() {

    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let mock_cfg = mock_test_config();
        let mock_chain_key = ChainKey::default().with_name("test_chain");
        let mut chain: ChainOfTrust = ChainOfTrust::new(&mock_cfg, &mock_chain_key).await.expect("Failed to create the chain of trust");

        let validator = RubberStampValidator::default();
        let evt_data = Event::new(PrimaryKey::generate(), DefaultMeta::default(), Bytes::default()).to_event_data();
        let _: Event<DefaultMeta> = chain.process(evt_data, validator).await.unwrap();
    });
}