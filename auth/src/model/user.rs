#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::*;
use ate::crypto::*;
use ate::prelude::*;

use super::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub email: String,
    pub person: DaoRef<Person>,
    pub accepted_terms: DaoRef<AcceptedTerms>,
    pub verification_code: Option<String>,
    pub uid: u32,
    pub role: UserRole,    
    pub status: UserStatus,
    pub last_login: Option<chrono::naive::NaiveDate>,
    pub access: Vec<Authorization>,
    pub foreign: DaoForeign,
    pub sudo: DaoRef<Sudo>,
    pub nominal_read: ate::crypto::AteHash,
    pub nominal_public_read: PublicEncryptKey,
    pub nominal_write: PublicSignKey,
    pub sudo_read: ate::crypto::AteHash,
    pub sudo_public_read: PublicEncryptKey,
    pub sudo_write: PublicSignKey,
}