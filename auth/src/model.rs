#![allow(unused_imports, dead_code)]
use serde::*;
use fxhash::FxHashMap;
use ate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Gender
{
    Unspecified,
    Male,
    Female,
    Other,
}

impl Default
for Gender
{
    fn default() -> Self {
        Gender::Unspecified
    }
}

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SmsVerification {
    pub salt: String,
    pub hash: AteHash,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmailVerification {
    pub code: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum SshKeyType
{
    DSA,
    RSA,
    ED25519,
    ECDSA
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AuthenticationMethod
{
    WithPrivateKey(PublicSignKey),
    WithPassword {
        salt: String,
        hash: AteHash,
    },
    WithAuthenticator {
        secret: String,
    },
    WithSmsAuthentication {
        salt: String,
        hash: AteHash,
    },
    WithEmailVerification {
        code: String,
    },
    WithSshKey {
        key_type: SshKeyType,
        secret: String,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum UserStatus
{
    Nominal,
    Unverified,
    Locked,
}

impl Default
for UserStatus
{
    fn default() -> Self {
        UserStatus::Nominal
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum UserRole {
    Human,
    Robot,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub person: DaoRef<Person>,
    pub role: UserRole,    
    pub status: UserStatus,
    pub last_login: Option<chrono::naive::NaiveDate>,
    pub access: Vec<Authorization>,
    pub foreign: DaoForeign,
    pub sudo: DaoRef<Sudo>,
    pub nominal_read: ate::crypto::Hash,
    pub nominal_public_read: PublicEncryptKey,
    pub nominal_write: PublicSignKey,
    pub sudo_read: ate::crypto::Hash,
    pub sudo_public_read: PublicEncryptKey,
    pub sudo_write: PublicSignKey,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Sudo {
    pub google_auth: String,
    pub secret: String,
    pub qr_code: String,
    pub access: Vec<Authorization>,
    pub groups: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Advert {
    pub email: String,
    pub nominal_encrypt: PublicEncryptKey,
    pub nominal_auth: PublicSignKey,
    pub sudo_encrypt: PublicEncryptKey,
    pub sudo_auth: PublicSignKey,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Authorization {
    pub read: EncryptKey,
    pub private_read: PrivateEncryptKey,
    pub write: PrivateSignKey,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Role {
    pub purpose: AteRolePurpose,
    pub access: MultiEncryptedSecureData<Authorization>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Group {
    pub name: String,
    pub roles: Vec<Role>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Company
{
    pub domain: String,
    pub registration_no: String,
    pub tax_id: String,
    pub phone_number: String,
    pub email: String,
    pub do_business_as: String,
    pub legal_business_name: String,
    pub share_holders: DaoVec<Person>,
}