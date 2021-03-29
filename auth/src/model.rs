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
    first_name: String,
    last_name: String,
    other_names: Vec<String>,
    date_of_birth: Option<chrono::naive::NaiveDate>,
    gender: Gender,
    nationalities: Vec<isocountry::CountryCode>,
    foreign: DaoForeign
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SmsVerification {
    salt: String,
    hash: AteHash,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmailVerification {
    code: String,
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
    WithPrivateKey(PublicKey),
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
    person: DaoRef<Person>,
    account: DaoRef<Account>,
    role: UserRole,    
    status: UserStatus,
    not_lockable: bool,
    failed_logins: i32,
    last_login: Option<chrono::naive::NaiveDate>,    
    login_methods: Vec<AuthenticationMethod>,
    access: Vec<Authorization>,
    foreign: DaoForeign
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Authorization {
    name: String,
    read: Option<EncryptKey>,
    write: Option<PrivateKey>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccountCore {
    email: String,
    name: String,
    access: Vec<Authorization>,
    foreign: DaoForeign
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccountPersonal
{
    core: AccountCore,
    user: DaoRef<User>,
    person: DaoRef<Person>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Company
{
    registration_no: String,
    tax_id: String,
    phone_number: String,
    email: String,
    do_business_as: String,
    legal_business_name: String,
    share_holders: DaoVec<Person>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccountCompany
{
    core: AccountCore,
    users: DaoVec<User>,
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