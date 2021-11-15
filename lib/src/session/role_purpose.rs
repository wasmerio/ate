#[allow(unused_imports)]
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum AteRolePurpose {
    Owner,
    Personal,
    Delegate,
    Contributor,
    Observer,
    Finance,
    WebServer,
    EdgeCompute,
    Other(String),
}

impl std::fmt::Display for AteRolePurpose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AteRolePurpose::Owner => write!(f, "owner"),
            AteRolePurpose::Personal => write!(f, "personal"),
            AteRolePurpose::Delegate => write!(f, "delegate"),
            AteRolePurpose::Contributor => write!(f, "contributor"),
            AteRolePurpose::Observer => write!(f, "observer"),
            AteRolePurpose::Finance => write!(f, "finance"),
            AteRolePurpose::WebServer => write!(f, "www"),
            AteRolePurpose::EdgeCompute => write!(f, "edge"),
            AteRolePurpose::Other(a) => write!(f, "other-{}", a),
        }
    }
}

impl std::str::FromStr for AteRolePurpose {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "owner" => Ok(AteRolePurpose::Owner),
            "personal" => Ok(AteRolePurpose::Personal),
            "delegate" => Ok(AteRolePurpose::Delegate),
            "contributor" => Ok(AteRolePurpose::Contributor),
            "observer" => Ok(AteRolePurpose::Observer),
            "finance" => Ok(AteRolePurpose::Finance),
            "www" => Ok(AteRolePurpose::WebServer),
            "edge" => Ok(AteRolePurpose::EdgeCompute),
            a if a.starts_with("other-") && a.len() > 6 => Ok(AteRolePurpose::Other(a["other-".len()..].to_string())),
            _ => Err("valid values are 'owner', 'personal', 'delegate', 'contributor', 'observer', 'www', 'edge' and 'other-'"),
        }
    }
}
