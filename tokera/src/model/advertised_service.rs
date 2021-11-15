use serde::*;
use std::time::Duration;

use super::*;

/// The commodity kind determines how this commodity will be listed
/// on the market interaction interface. This metadata helps users
/// identify what they are subscribing for.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AdvertisedService {
    /// Code assigned to this service
    pub code: String,
    /// The name of the service as seen by all consumers
    pub name: String,
    /// Detailed description of what the service is
    pub description: String,
    /// These charges that will be applied to your account for this service
    /// if you subscribe to it
    pub rate_cards: Vec<RateCard>,
    /// Identity of the owner of this particular service (the owner of
    /// the service is able to incur charges on your wallet)
    pub owner_identity: String,
    /// Terms and conditions for consuming this service
    pub terms_and_conditions: String,
    /// Amount of time before the services will be suspended
    pub grace_period: Duration,
    /// The subscription may still have some throttles
    pub throttle: Option<ThrottleTriggers>,
}
