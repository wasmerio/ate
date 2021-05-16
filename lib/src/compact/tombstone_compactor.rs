use fxhash::FxHashSet;

use crate::header::*;
use crate::meta::*;
use crate::event::*;
use crate::crypto::AteHash;

use super::*;

#[derive(Default, Clone)]
pub struct TombstoneCompactor
{
    ignored: FxHashSet<AteHash>,
    tombstoned: FxHashSet<PrimaryKey>,
}

impl EventCompactor
for TombstoneCompactor
{
    fn clone_compactor(&self) -> Option<Box<dyn EventCompactor>> {
        Some(Box::new(Self::default()))
    }
    
    fn relevance(&self, header: &EventHeader) -> EventRelevance
    {
        let key = match header.meta.get_data_key() {
            Some(key) => key,
            None => { return EventRelevance::Abstain; }
        };

        if self.ignored.contains(&header.raw.event_hash) {
            return EventRelevance::Abstain;
        }

        match self.tombstoned.contains(&key) {
            true => EventRelevance::ForceDrop,
            false => EventRelevance::Abstain,
        }        
    }

    fn feed(&mut self, header: &EventHeader, _keep: bool) {
        if let Some(key) = header.meta.get_tombstone() {
            self.tombstoned.insert(key.clone());
        } else if let Some(key) = header.meta.get_data_key() {
            if self.tombstoned.contains(&key) == false {
                self.ignored.insert(header.raw.event_hash);
            }
        }
    }
    
    fn name(&self) -> &str {
        "tombstone-compactor"
    }
}

impl Metadata
{
    pub fn get_tombstone(&self) -> Option<PrimaryKey> {
        self.core.iter().filter_map(
            |m| {
                match m
                {
                    CoreMetadata::Tombstone(k) => Some(k.clone()),
                     _ => None
                }
            }
        )
        .next()
    }

    pub fn add_tombstone(&mut self, key: PrimaryKey) {
        let has = self.core.iter().any(
            |m| {
                match m {
                    CoreMetadata::Tombstone(k) => *k == key,
                     _ => false
                }
            }
        );
        if has == true { return; }
        self.core.push(CoreMetadata::Tombstone(key));
    }
}