use std::{fmt::Display, str::FromStr};

#[derive(Debug, Clone, Copy)]
pub enum SerializationFormat {
    Json,
    Bincode,
}

impl FromStr for SerializationFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "json" => Ok(SerializationFormat::Json),
            "bincode" => Ok(SerializationFormat::Bincode),
            _ => {
                let msg = "valid serialization formats are 'json' and 'bincode'";
                return Err(msg.to_string());
            }
        }
    }
}

impl Display for SerializationFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SerializationFormat::Json => write!(f, "json"),
            SerializationFormat::Bincode => write!(f, "bincode"),
        }
    }
}
