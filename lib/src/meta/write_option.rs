use fxhash::FxHashSet;
use serde::{Serialize, Deserialize};

use crate::crypto::*;

/// Determines who is allowed to attach events records to this part of the
/// chain-of-trust key. Only users who have the `PrivateKey` in their session
/// will be able to write these records to the chain. The hash of the `PublicKey`
/// side is stored in this enum.
#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum WriteOption
{
    Inherit,
    Everyone,
    Nobody,
    Specific(AteHash),
    Any(Vec<AteHash>)
}

impl WriteOption
{
    pub fn vals(&self) -> FxHashSet<AteHash> {
        let mut ret = FxHashSet::default();
        match self {
            WriteOption::Specific(a) => { ret.insert(a.clone()); }
            WriteOption::Any(hashes) => {
                for a in hashes {
                    ret.insert(a.clone());
                }
            },
            _ => {}
        }
        return ret;
    }

    pub fn or(self, other: &WriteOption) -> WriteOption {
        match other {
            WriteOption::Inherit => self,
            WriteOption::Any(keys) => {
                let mut vals = self.vals();
                for a in keys {
                    vals.insert(a.clone());
                }
                WriteOption::Any(vals.iter().map(|k| k.clone()).collect::<Vec<_>>())
            },
            WriteOption::Specific(hash) => {
                let mut vals = self.vals();
                vals.insert(hash.clone());
                let vals = vals.iter().map(|k| k.clone()).collect::<Vec<_>>();
                match vals.len() {
                    1 => WriteOption::Specific(vals.into_iter().next().unwrap()),
                    _ => WriteOption::Any(vals)
                }
            },
            a => a.clone(),
        }
    }
}

impl Default
for WriteOption
{
    fn default() -> WriteOption {
        WriteOption::Inherit
    }
}

impl std::fmt::Display
for WriteOption {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            WriteOption::Everyone => {
                write!(f, "everyone")
            },
            WriteOption::Any(vec) => {
                write!(f, "any(")?;
                let mut first = true;
                for hash in vec {
                    if first == true {
                        first = false;
                    } else {
                        write!(f, ",")?;
                    }
                    write!(f, "{}", hash)?;
                }
                write!(f, ")")
            },
            WriteOption::Inherit => {
                write!(f, "inherit")
            },
            WriteOption::Nobody => {
                write!(f, "nobody")
            },
            WriteOption::Specific(hash) => {
                write!(f, "specifc({})", hash)
            },
        }
    }
}