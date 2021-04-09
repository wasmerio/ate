#![allow(unused_imports, dead_code)]
use serde::*;
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
    pub account: DaoRef<Account>,
    pub role: UserRole,    
    pub status: UserStatus,
    pub last_login: Option<chrono::naive::NaiveDate>,
    pub access: Vec<Authorization>,
    pub foreign: DaoForeign,
    pub sudo: DaoRef<Sudo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Sudo {
    pub google_auth: String,
    pub qr_code: String,
    pub access: Vec<Authorization>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Authorization {
    pub name: String,
    pub read: Option<EncryptKey>,
    pub write: Option<PrivateSignKey>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccountCore {
    pub email: String,
    pub name: String,
    pub access: Vec<Authorization>,
    pub foreign: DaoForeign
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccountPersonal
{
    pub core: AccountCore,
    pub user: DaoRef<User>,
    pub person: DaoRef<Person>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Company
{
    pub registration_no: String,
    pub tax_id: String,
    pub phone_number: String,
    pub email: String,
    pub do_business_as: String,
    pub legal_business_name: String,
    pub share_holders: DaoVec<Person>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccountCompany
{
    pub core: AccountCore,
    pub users: DaoVec<User>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Account
{
    Company(AccountCompany),
    Personal(AccountPersonal)
}

impl Account
{
    pub fn core(&self) -> &AccountCore {
        match self {
            Account::Company(a) => &a.core,
            Account::Personal(a) => &a.core,
        }
    }
}