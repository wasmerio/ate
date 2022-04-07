use crate::spec::*;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use super::*;

/// Determines what optimizes and defaults ATE selects based of a particular
/// group of usecases
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfiguredFor {
    /// ATE is left completely unconfigured with no-assumptions and no default functionality
    Raw,
    /// ATE is configured with the minimum that is considered at least functional
    Barebone,
    /// ATE will optimize its usage for the redo-logs with the smallest size possible, this
    /// includes using compression on the data streams by default.
    SmallestSize,
    /// ATE will use serializers that are much faster than normal however they do not support
    /// forward or backwards compatibility meaning changes to the data object schemas will
    /// break your trees thus you will need to handle versioning yourself manually.
    BestPerformance,
    /// ATE will use serializers that provide both forward and backward compatibility for changes
    /// to the metadata schema and the data schema. This format while slower than the performance
    /// setting allows seamless upgrades and changes to your model without breaking existing data.
    BestCompatibility,
    /// A balance between performance, compatibility and security that gives a bit of each without
    /// without going towards the extremes of any. For instance, the data model is forwards and
    /// backwards compatible however the metadata is not. Encryption is good eno\for all known
    /// attacks of today but less protected against unknown attacks of the future.
    Balanced,
    /// Provides the best encryption routines available at the expense of performance and size
    BestSecurity,
}

impl ConfiguredFor {
    pub fn ntp_tolerance(&self) -> u32 {
        match self {
            ConfiguredFor::BestPerformance => 60000u32,
            ConfiguredFor::BestSecurity => 30000u32,
            _ => 40000u32,
        }
    }
}

impl std::str::FromStr for ConfiguredFor {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "raw" => Ok(ConfiguredFor::Raw),
            "barebone" => Ok(ConfiguredFor::Barebone),
            "best_performance" => Ok(ConfiguredFor::BestPerformance),
            "performance" => Ok(ConfiguredFor::BestPerformance),
            "speed" => Ok(ConfiguredFor::BestPerformance),
            "best_compatibility" => Ok(ConfiguredFor::BestCompatibility),
            "compatibility" => Ok(ConfiguredFor::BestCompatibility),
            "balanced" => Ok(ConfiguredFor::Balanced),
            "best_security" => Ok(ConfiguredFor::BestSecurity),
            "security" => Ok(ConfiguredFor::BestSecurity),
            _ => Err("valid values are 'raw', 'barebone', 'best_performance', 'performance', 'speed', 'best_compatibility', 'compatibility', 'balanced', 'best_security' and 'security'"),
        }
    }
}

impl Default for ConfiguredFor {
    fn default() -> ConfiguredFor {
        ConfiguredFor::Balanced
    }
}

impl ConfAte {
    pub fn configured_for(&mut self, configured_for: ConfiguredFor) {
        self.configured_for = configured_for;

        match configured_for {
            ConfiguredFor::BestPerformance => {
                self.log_format.meta = SerializationFormat::Bincode;
                self.log_format.data = SerializationFormat::Bincode;
            }
            ConfiguredFor::BestCompatibility => {
                self.log_format.meta = SerializationFormat::Json;
                self.log_format.data = SerializationFormat::Json;
            }
            _ => {
                self.log_format.meta = SerializationFormat::Bincode;
                self.log_format.data = SerializationFormat::Json;
            }
        }
    }
}
