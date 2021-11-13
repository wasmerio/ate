use serde::*;

use super::*;

/// The commodity kind determines how this commodity will be listed
/// on the market interaction interface. This metadata helps users
/// identify what they are trading.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommodityKindType
{
    pub name: String,
    pub category: CommodityCategory,
    pub description: Option<String>,
    pub details: Option<String>,
    pub image: Option<Vec<u8>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CommodityKind
{
    Coin(NationalCurrency),
}

impl CommodityKind
{
    pub fn params(&self) -> CommodityKindType {
        match self {
            CommodityKind::Coin(c) => CommodityKindType {
                name: "Coin".to_string(),
                category: CommodityCategory::NationalCurrency(c.clone()),
                description: None,
                details: None,
                image: None,
            },
        }
    }

    pub fn name(&self) -> String {
        self.params().name
    }

    pub fn category(&self) -> CommodityCategory {
        self.params().category
    }

    pub fn description(&self) -> Option<String> {
        self.params().description
    }

    pub fn details(&self) -> Option<String> {
        self.params().details
    }

    pub fn image(&self) -> Option<Vec<u8>> {
        self.params().image
    }
}