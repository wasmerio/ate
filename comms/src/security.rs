#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StreamSecurity {
    Unencrypted,
    AnyEncryption,
    ClassicEncryption,
    QuantumEncryption,
    DoubleEncryption,
}

impl Default
for StreamSecurity {
    fn default() -> Self {
        StreamSecurity::AnyEncryption
    }
}

impl StreamSecurity {
    pub fn classic_encryption(&self) -> bool {
        match self {
            StreamSecurity::ClassicEncryption |
            StreamSecurity::DoubleEncryption => {
                true
            },
            _ => false
        }
    }

    pub fn quantum_encryption(&self, https: bool) -> bool {
        match self {
            StreamSecurity::AnyEncryption => {
                https == false
            }
            StreamSecurity::QuantumEncryption |
            StreamSecurity::DoubleEncryption => {
                true
            },
            _ => false
        }
    }
}

impl std::str::FromStr
for StreamSecurity
{
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(
            match s {
                "none" | "" | "no" | "unencrypted" => StreamSecurity::Unencrypted,
                "any" | "anyencryption" | "encrypted" | "any_encryption" => {
                    StreamSecurity::AnyEncryption
                },
                "classic" => StreamSecurity::ClassicEncryption,
                "quantum" => StreamSecurity::QuantumEncryption,
                "double" => StreamSecurity::DoubleEncryption,
                a => {
                    return Err(format!("stream security type ({}) is not valid - try: 'none', 'any', 'classic', 'quantum' or 'double'", a))
                }
            }
        )
    }
}
