#[allow(unused_imports)]
use serde::{Serialize, Deserialize, de::DeserializeOwned};

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum AteRolePurpose
{
    Owner,
    Delegate,
    Contributor,
    Observer,
    Other(String),
}

impl std::fmt::Display
for AteRolePurpose
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AteRolePurpose::Owner => write!(f, "owner"),
            AteRolePurpose::Delegate => write!(f, "delegate"),
            AteRolePurpose::Contributor => write!(f, "contributor"),
            AteRolePurpose::Observer => write!(f, "observer"),
            AteRolePurpose::Other(a) => write!(f, "other-{}", a),
        }
    }
}

impl std::str::FromStr
for AteRolePurpose
{
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "owner" => Ok(AteRolePurpose::Owner),
            "delegate" => Ok(AteRolePurpose::Delegate),
            "contributor" => Ok(AteRolePurpose::Contributor),
            "observer" => Ok(AteRolePurpose::Observer),
            a if a.starts_with("other-") && a.len() > 6 => Ok(AteRolePurpose::Other(a["other-".len()..].to_string())),
            _ => Err("valid values are 'owner', 'delegate', 'contributor', 'observer' and 'other-'"),
        }
    }
}