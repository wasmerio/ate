use serde::{Deserialize, Serialize};

use super::*;

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash, PartialEq, Eq)]
pub struct MetaAuthorization {
    pub read: ReadOption,
    pub write: WriteOption,
}

impl MetaAuthorization {
    pub fn is_relevant(&self) -> bool {
        self.read != ReadOption::Inherit || self.write != WriteOption::Inherit
    }
}

impl std::fmt::Display for MetaAuthorization {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let r = match &self.read {
            ReadOption::Everyone(key) => {
                if let Some(key) = key {
                    format!("everyone({})", key.hash())
                } else {
                    "everyone".to_string()
                }
            }
            ReadOption::Inherit => "inherit".to_string(),
            ReadOption::Specific(a, _derived) => format!("specific-{}", a),
        };
        let w = match &self.write {
            WriteOption::Everyone => "everyone".to_string(),
            WriteOption::Nobody => "nobody".to_string(),
            WriteOption::Inherit => "inherit".to_string(),
            WriteOption::Specific(a) => format!("specific-{}", a),
            WriteOption::Any(a) => {
                let mut r = "any".to_string();
                for a in a {
                    r.push_str("-");
                    r.push_str(a.to_string().as_str());
                }
                r
            }
        };
        write!(f, "(r:{}, w:{})", r, w)
    }
}
