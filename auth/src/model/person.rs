#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::*;
use ate::prelude::*;

use super::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Person {
    pub first_name: String,
    pub last_name: String,
    pub other_names: Vec<String>,
    pub date_of_birth: Option<chrono::naive::NaiveDate>,
    pub gender: Gender,
    pub nationalities: Vec<isocountry::CountryCode>,
    pub foreign: DaoForeign
}