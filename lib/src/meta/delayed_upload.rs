use serde::{Deserialize, Serialize};

use crate::time::ChainTimestamp;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaDelayedUpload {
    pub complete: bool,
    pub from: ChainTimestamp,
    pub to: ChainTimestamp,
}

impl std::fmt::Display for MetaDelayedUpload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "from-{}-to-{}", self.from, self.to)?;
        if self.complete {
            write!(f, "-complete")?;
        } else {
            write!(f, "-incomplete")?;
        }
        Ok(())
    }
}
