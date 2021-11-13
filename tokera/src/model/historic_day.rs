use serde::*;

use super::*;

/// Represents a day of activity that has happened
/// in ones account
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HistoricDay
{
    // Which day does this data relate to
    pub day: u32,
    // Represents an activity that has occured on this day
    pub activities: Vec<HistoricActivity>,
}