use serde::*;
use std::fmt;

use crate::model::Decimal;
use super::*;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChargeMetric
{
    DownloadBandwidth,
    UploadBandwidth,
    DataStorage,
    Compute,
}

impl fmt::Display
for ChargeMetric
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChargeMetric::DownloadBandwidth => write!(f, "downloaded bandwidth"),
            ChargeMetric::UploadBandwidth => write!(f, "uploaded bandwidth"),
            ChargeMetric::DataStorage => write!(f, "data storage usage"),
            ChargeMetric::Compute => write!(f, "compute usage"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChargeUnits
{
    FlatRate,

    Seconds(ChargeMetric),
    Minutes(ChargeMetric),
    Hours(ChargeMetric),
    Days(ChargeMetric),
    Weeks(ChargeMetric),

    Bytes(ChargeMetric),
    KiloBytes(ChargeMetric),
    MegaBytes(ChargeMetric),
    GigaBytes(ChargeMetric),
    TeraBytes(ChargeMetric),
    PetaBytes(ChargeMetric),
}

impl ChargeUnits
{
    pub fn scale(&self) -> Decimal
    {
        match self {
            ChargeUnits::FlatRate => Decimal::from(1u64),

            ChargeUnits::Bytes(_) => Decimal::from(1u64),
            ChargeUnits::KiloBytes(_) => Decimal::from(1000u64),
            ChargeUnits::MegaBytes(_) => Decimal::from(1000000u64),
            ChargeUnits::GigaBytes(_) => Decimal::from(1000000000u64),
            ChargeUnits::TeraBytes(_) => Decimal::from(1000000000000u64),
            ChargeUnits::PetaBytes(_) => Decimal::from(1000000000000000u64),

            ChargeUnits::Seconds(_) => Decimal::from(1u64),
            ChargeUnits::Minutes(_) => Decimal::from(60u64),
            ChargeUnits::Hours(_) => Decimal::from(3600u64),
            ChargeUnits::Days(_) => Decimal::from(86400u64),
            ChargeUnits::Weeks(_) => Decimal::from(604800u64),
        }
    }

    pub fn abbreviation(&self) -> &'static str {
        match self {
            ChargeUnits::FlatRate => "flat",

            ChargeUnits::Bytes(_) => "B",
            ChargeUnits::KiloBytes(_) => "KB",
            ChargeUnits::MegaBytes(_) => "MB",
            ChargeUnits::GigaBytes(_) => "GB",
            ChargeUnits::TeraBytes(_) => "TB",
            ChargeUnits::PetaBytes(_) => "PB",

            ChargeUnits::Seconds(_) => "s",
            ChargeUnits::Minutes(_) => "m",
            ChargeUnits::Hours(_) => "h",
            ChargeUnits::Days(_) => "d",
            ChargeUnits::Weeks(_) => "w",
        }
    }

    pub fn metric(&self) -> Option<ChargeMetric>
    {
        match self {
            ChargeUnits::FlatRate => None,

            ChargeUnits::Bytes(a) => Some(a.clone()),
            ChargeUnits::KiloBytes(a) => Some(a.clone()),
            ChargeUnits::MegaBytes(a) => Some(a.clone()),
            ChargeUnits::GigaBytes(a) => Some(a.clone()),
            ChargeUnits::TeraBytes(a) => Some(a.clone()),
            ChargeUnits::PetaBytes(a) => Some(a.clone()),

            ChargeUnits::Seconds(a) => Some(a.clone()),
            ChargeUnits::Minutes(a) => Some(a.clone()),
            ChargeUnits::Hours(a) => Some(a.clone()),
            ChargeUnits::Days(a) => Some(a.clone()),
            ChargeUnits::Weeks(a) => Some(a.clone()),
        }
    }
}

impl fmt::Display
for ChargeUnits
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChargeUnits::FlatRate => write!(f, "flat"),
            
            ChargeUnits::Bytes(a) => write!(f, "bytes {}", a),
            ChargeUnits::KiloBytes(a) => write!(f, "kilobytes {}", a),
            ChargeUnits::MegaBytes(a) => write!(f, "megabytes {}", a),
            ChargeUnits::GigaBytes(a) => write!(f, "gigabytes {}", a),
            ChargeUnits::TeraBytes(a) => write!(f, "terabytes {}", a),
            ChargeUnits::PetaBytes(a) => write!(f, "petabytes {}", a),

            ChargeUnits::Seconds(a) => write!(f, "secs {}", a),
            ChargeUnits::Minutes(a) => write!(f, "mins {}", a),
            ChargeUnits::Hours(a) => write!(f, "hours {}", a),
            ChargeUnits::Days(a) => write!(f, "days {}", a),
            ChargeUnits::Weeks(a) => write!(f, "weeks {}", a),
        }
    }
}

/// Represents a particular charge that will be applied to your
/// wallet thats triggered at specific trigger points
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Charge
{
    pub amount: Decimal,
    pub units: ChargeUnits,
    pub frequency: ChargeFrequency,
}

impl fmt::Display
for Charge
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let freq = self.frequency.to_string().replace("-", " ");
        write!(f, "{} per {} {}", self.amount, self.units, freq)
    }
}