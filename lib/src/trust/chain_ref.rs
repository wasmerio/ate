#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::{Serialize, Deserialize};
use url::Url;

use super::ChainKey;

/// Unique reference to a particular chain-of-trust. The design must
/// partition their data space into seperate chains to improve scalability
/// and performance as a single chain will reside on a single node within
/// the cluster.
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChainRef {
    pub url: Url,
    pub key: ChainKey,
}

impl std::fmt::Display
for ChainRef {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}/{}", self.url, self.key)
    }
}

impl ChainRef
{
    pub fn new(url: url::Url, key: ChainKey) -> ChainRef {
        ChainRef {
            url: url,
            key: key,
        }
    }

    pub fn to_string(&self) -> String
    {
        format!("{}/{}", self.url, self.key)
    }
}