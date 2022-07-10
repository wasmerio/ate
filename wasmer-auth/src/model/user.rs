use ate::crypto::*;
use ate::prelude::*;
use serde::*;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use super::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub email: String,
    pub person: DaoChild<Person>,
    pub accepted_terms: DaoChild<AcceptedTerms>,
    pub verification_code: Option<String>,
    pub uid: u32,
    pub role: UserRole,
    pub status: UserStatus,
    pub last_login: Option<chrono::naive::NaiveDate>,
    pub access: Vec<Authorization>,
    pub foreign: DaoForeign,
    pub sudo: DaoChild<Sudo>,
    pub advert: DaoChild<Advert>,
    pub recovery: DaoChild<UserRecovery>,
    pub nominal_read: ate::crypto::AteHash,
    pub nominal_public_read: PublicEncryptKey,
    pub nominal_write: PublicSignKey,
    pub sudo_read: ate::crypto::AteHash,
    pub sudo_public_read: PublicEncryptKey,
    pub sudo_write: PublicSignKey,
    pub broker_read: PrivateEncryptKey,
    pub broker_write: PrivateSignKey,
}
