use serde::*;

use super::*;

/// The commodity category allows the buyers to easily
/// find, search and filter what they specifically want to buy.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum CommodityCategory {
    /// National currency backed by a national government
    NationalCurrency(NationalCurrency),
    /// Digital asset such as a game account/item
    DigitalAsset(DigitalAsset),
    /// Digital service such as a subscription
    DigitalService(DigitalService),
}

impl CommodityCategory {
    pub fn name(&self) -> &str {
        self.params().0
    }

    pub fn description(&self) -> String {
        self.params().1
    }

    fn params(&self) -> (&str, String) {
        match self {
            CommodityCategory::NationalCurrency(a) => (a.name(), a.description()),
            CommodityCategory::DigitalAsset(a) => (a.name(), a.description().to_string()),
            CommodityCategory::DigitalService(a) => (a.name(), a.description().to_string()),
        }
    }
}
