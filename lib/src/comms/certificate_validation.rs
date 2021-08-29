use crate::crypto::AteHash;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CertificateValidation
{
    DenyAll,
    AllowAll,
    AllowedCertificates(Vec<AteHash>),
}

impl CertificateValidation
{
    pub fn validate(&self, cert: &AteHash) -> bool
    {
        match self {
            CertificateValidation::DenyAll => false,
            CertificateValidation::AllowAll => true,
            CertificateValidation::AllowedCertificates(a) => {
                a.contains(cert)
            }
        }
    }
}