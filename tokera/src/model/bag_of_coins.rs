use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use tracing::{debug, error, info, warn};

use crate::model::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BagOfCoins {
    pub coins: Vec<CarvedCoin>,
}

impl Default for BagOfCoins {
    fn default() -> BagOfCoins {
        BagOfCoins { coins: Vec::new() }
    }
}

impl BagOfCoins {
    pub async fn to_ownerships(&self) -> Vec<Ownership> {
        let mut owners = self.coins.iter().map(|a| &a.owner).collect::<Vec<_>>();
        owners.sort_by(|a, b| a.cmp(b));
        owners.dedup_by(|a, b| (*a).eq(b));

        let owners = owners.into_iter().map(|a| a.clone()).collect::<Vec<_>>();
        owners
    }
}
