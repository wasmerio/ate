use serde::{Deserialize, Serialize};
use std::result::Result;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

/// Size of a cryptographic key, smaller keys are still very secure but
/// have less room in the future should new attacks be found against the
/// crypto algorithms used by ATE.
#[repr(u8)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KeySize {
    #[allow(dead_code)]
    Bit128 = 16,
    #[allow(dead_code)]
    Bit192 = 24,
    #[allow(dead_code)]
    Bit256 = 32,
}

impl KeySize {
    pub fn as_str(&self) -> &str {
        match &self {
            KeySize::Bit128 => "128",
            KeySize::Bit192 => "192",
            KeySize::Bit256 => "256",
        }
    }
}

impl std::str::FromStr for KeySize {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "128" => Ok(KeySize::Bit128),
            "192" => Ok(KeySize::Bit192),
            "256" => Ok(KeySize::Bit256),
            _ => Err("valid values are '128', '192', '256'"),
        }
    }
}

impl std::fmt::Display for KeySize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeySize::Bit128 => write!(f, "128"),
            KeySize::Bit192 => write!(f, "192"),
            KeySize::Bit256 => write!(f, "256"),
        }
    }
}
