use serde::*;
use ate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Certificate
{
    pub data: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CertificateKey
{
    pub domain: String,
    pub certs: Vec<Certificate>,
    pub pk: Vec<u8>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct CertificateStore
{
    pub certs: DaoVec<CertificateKey>,
}