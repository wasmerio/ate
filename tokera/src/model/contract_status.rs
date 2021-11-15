use chrono::DateTime;
use chrono::Utc;
use serde::*;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ThrottleTriggers {
    pub download_per_second: Option<u64>,
    pub upload_per_second: Option<u64>,
    pub read_only_threshold: Option<u64>,
}

/// The contract status determines if aggrements are being honoured
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ContractStatus {
    MissingContract {
        throttle: ThrottleTriggers,
    },
    Nominal {
        throttle: ThrottleTriggers,
    },
    InDefault {
        since: DateTime<Utc>,
        throttle: ThrottleTriggers,
    },
}

impl ContractStatus {
    pub fn throttle(&self) -> &ThrottleTriggers {
        match self {
            ContractStatus::Nominal { throttle } => throttle,
            ContractStatus::MissingContract { throttle } => throttle,
            ContractStatus::InDefault { since: _, throttle } => throttle,
        }
    }
}

impl std::fmt::Display for ContractStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContractStatus::Nominal { throttle: _ } => write!(f, "nominal"),
            ContractStatus::MissingContract { throttle: _ } => write!(f, "missing"),
            ContractStatus::InDefault { since, throttle: _ } => {
                write!(f, "default-since-{}", since)
            }
        }
    }
}
