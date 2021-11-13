use serde::*;

use super::*;

/// Rate cards represent a series of charges incurred for consumption
/// of various services and or commodities.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RateCard
{
    /// The currency that the rate card make charges at
    pub currency: NationalCurrency,

    // List of all the charges associated with this rate card
    pub charges: Vec<Charge>,
}