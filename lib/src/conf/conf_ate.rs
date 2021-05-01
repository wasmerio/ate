#[allow(unused_imports)]
use log::{info, error, debug};

use crate::crypto::KeySize;
use crate::spec::*;
use crate::mesh::RecoveryMode;

use super::*;

/// Configuration settings for the ATE datastore
///
#[derive(Debug, Clone)]
pub struct ConfAte
{
    /// Optimizes ATE for a specific group of usecases
    pub(super) configured_for: ConfiguredFor,

    /// Specified the recovery mode that the mesh will take
    pub recovery_mode: RecoveryMode,

    /// Directory path that the redo logs will be stored.
    pub log_path: String,

    /// NTP pool server which ATE will synchronize its clocks with, its
    /// important to have synchronized clocks with ATE as it uses time as
    /// digest to prevent replay attacks
    pub ntp_pool: String,
    /// Port that the NTP server is listening on (defaults to 123)
    pub ntp_port: u16,
    /// Flag that indicates if the time keeper will sync with NTP or not
    /// (avoiding NTP sync means one can run fully offline but time drift
    ///  will cause issues with multi factor authentication and timestamps)
    pub ntp_sync: bool,

    /// Flag that determines if ATE will use DNSSec or just plain DNS
    pub dns_sec: bool,
    /// DNS server that queries will be made do by the chain registry
    pub dns_server: String,

    /// Flag that indicates if encryption will be used for the underlying
    /// connections over the wire. When using a ATE's in built encryption
    /// and quantum resistant signatures it is not mandatory to use
    /// wire encryption as confidentially and integrity are already enforced however
    /// for best security it is advisable to apply a layered defence, of
    /// which double encrypting your data and the metadata around it is
    /// another defence.
    pub wire_encryption: Option<KeySize>,

    /// Size of the buffer on mesh clients, tweak this number with care
    pub buffer_size_client: usize,
    /// Size of the buffer on mesh servers, tweak this number with care
    pub buffer_size_server: usize,

    /// Size of the local cache that stores redo log entries in memory
    pub load_cache_size: usize,
    /// Number of seconds that redo log entries will remain in memory before
    /// they are evicted
    pub load_cache_ttl: u64,

    /// Serialization format of the log files
    pub log_format: MessageFormat,
    /// Serialization format of the data on the network pipes between nodes and clients
    pub wire_format: SerializationFormat,
}

impl Default
for ConfAte
{
    fn default() -> ConfAte {
        ConfAte {
            log_path: "/tmp/ate".to_string(),
            dns_sec: false,
            dns_server: "8.8.8.8".to_string(),
            recovery_mode: RecoveryMode::ReadOnlyAsync,
            ntp_sync: true,
            ntp_pool: "pool.ntp.org".to_string(),
            ntp_port: 123,
            wire_encryption: Some(KeySize::Bit128),
            configured_for: ConfiguredFor::default(),
            buffer_size_client: 2,
            buffer_size_server: 10,
            load_cache_size: 1000,
            load_cache_ttl: 30,
            log_format: MessageFormat {
                meta: SerializationFormat::Bincode,
                data: SerializationFormat::Json,
            },
            wire_format: SerializationFormat::Bincode,
        }
    }
}