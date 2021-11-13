use serde::*;

use ate::prelude::*;
use super::*;

/// Represents a month of activity that has happened
/// in ones account
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HistoricMonth
{
    // Which month does this data relate to
    pub month: u32,
    // The year that this history occured
    pub year: i32,
    /// Represents everything that happened within this month
    pub days: DaoVec<HistoricDay>
}