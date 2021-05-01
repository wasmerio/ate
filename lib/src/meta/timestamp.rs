use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaTimestamp
{
    pub time_since_epoch_ms: u64,
}

impl std::fmt::Display
for MetaTimestamp
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}ms", self.time_since_epoch_ms)
    }
}