use ate::prelude::*;
use serde::*;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use super::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Company {
    pub domain: String,
    pub registration_no: String,
    pub tax_id: String,
    pub phone_number: String,
    pub email: String,
    pub do_business_as: String,
    pub legal_business_name: String,
    pub share_holders: DaoVec<Person>,
}
