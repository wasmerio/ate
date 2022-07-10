use serde::*;

use super::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DigitalService {
    // Service provider will offer their services iteratively as a subscription
    Subscription,
    /// Time executing some automation for a particular digital task
    AutomationTime(AutomationTime),
}

impl DigitalService {
    pub fn name(&self) -> &str {
        self.params().0
    }

    pub fn description(&self) -> &str {
        self.params().1
    }

    fn params(&self) -> (&str, &str) {
        match self {
            DigitalService::Subscription => (
                "Subscription",
                "Service provider will offer their services iteratively as a subscription.",
            ),
            DigitalService::AutomationTime(a) => (a.name(), a.description()),
        }
    }
}
