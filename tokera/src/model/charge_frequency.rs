use serde::*;
use chrono::Duration;
use std::fmt;

/// Determines the frequency that you will be charged
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChargeFrequency
{
    Once,
    PerMinute,
    PerHour,
    PerDay,
    PerWeek,
    PerMonth,
    PerYear,
}

impl ChargeFrequency
{
    pub fn as_duration(&self) -> Duration
    {
        match self {
            ChargeFrequency::Once => Duration::max_value(),
            ChargeFrequency::PerMinute => Duration::seconds(60),
            ChargeFrequency::PerHour => Duration::hours(1),
            ChargeFrequency::PerDay => Duration::days(1),
            ChargeFrequency::PerWeek => Duration::days(7),
            ChargeFrequency::PerMonth => Duration::days(30)
                .checked_add(&Duration::hours(10)).unwrap()
                .checked_add(&Duration::seconds(2)).unwrap()
                .checked_add(&Duration::milliseconds(880)).unwrap(),
            ChargeFrequency::PerYear => Duration::hours(8760),
        }
    }
}
impl fmt::Display
for ChargeFrequency
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChargeFrequency::Once => write!(f, "once-off"),
            ChargeFrequency::PerMinute => write!(f, "per-minute"),
            ChargeFrequency::PerHour => write!(f, "per-hour"),
            ChargeFrequency::PerDay => write!(f, "per-day"),
            ChargeFrequency::PerWeek => write!(f, "per-week"),
            ChargeFrequency::PerMonth => write!(f, "per-month"),
            ChargeFrequency::PerYear => write!(f, "per-year"),
        }
    }
}