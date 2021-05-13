use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Hash, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChainTimestamp
{
    pub time_since_epoch_ms: u64
}

impl From<u64>
for ChainTimestamp
{
    fn from(val: u64) -> ChainTimestamp
    {
        ChainTimestamp {
            time_since_epoch_ms: val,
        }
    }
}

impl std::fmt::Display
for ChainTimestamp
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}ms", self.time_since_epoch_ms)
    }
}