use crate::header::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct MetaCollection {
    pub parent_id: PrimaryKey,
    pub collection_id: u64,
}

impl std::fmt::Display for MetaCollection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.parent_id)?;
        if self.collection_id > 1 {
            write!(f, ".{}", self.collection_id)?;
        }
        Ok(())
    }
}
