use serde::{Serialize, Deserialize};

use super::*;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct MetaParent
{
    pub vec: MetaCollection,
}

impl std::fmt::Display
for MetaParent
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.vec.parent_id)?;
        if self.vec.collection_id > 1 {
            write!(f, "+col={}", self.vec.collection_id)?;
        }
        Ok(())
    }
}