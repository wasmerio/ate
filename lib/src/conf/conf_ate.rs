#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use std::time::Duration;

use crate::spec::*;
use crate::mesh::RecoveryMode;
use crate::mesh::BackupMode;
use crate::compact::CompactMode;

use super::*;

/// Configuration settings for the ATE datastore
///
#[derive(Debug, Clone)]
pub struct ConfAte
{
    /// Optimizes ATE for a specific group of usecases.
    pub(super) configured_for: ConfiguredFor,

    /// Specifies the recovery mode that the mesh will take.
    pub recovery_mode: RecoveryMode,

    /// Specifies the log compaction mode for the redo log.
    pub compact_mode: CompactMode,
    /// Compacts the redo log on bootstrapping of the program.
    pub compact_bootstrap: bool,
    /// Compacts the redo log on cleanup
    pub compact_cleanup: bool,

    /// Directory path that the redo logs will be stored.
    /// (if this option is none then the logs will be stored in memory)
    #[cfg(feature = "enable_local_fs")]
    pub log_path: Option<String>,

    /// Directory path that the backup files will be stored and fetched.
    /// (if this option is none then the logs will not be backed up)
    #[cfg(feature = "enable_local_fs")]
    pub backup_path: Option<String>,

    /// Specifies the backup mode that the mesh will undertake
    pub backup_mode: BackupMode,

    /// NTP pool server which ATE will synchronize its clocks with, its
    /// important to have synchronized clocks with ATE as it uses time as
    /// digest to prevent replay attacks
    #[cfg(feature = "enable_ntp")]
    pub ntp_pool: String,
    /// Port that the NTP server is listening on (defaults to 123)
    #[cfg(feature = "enable_ntp")]
    pub ntp_port: u16,
    /// Flag that indicates if the time keeper will sync with NTP or not
    /// (avoiding NTP sync means one can run fully offline but time drift
    ///  will cause issues with multi factor authentication and timestamps)
    #[cfg(feature = "enable_ntp")]
    pub ntp_sync: bool,

    /// Flag that determines if ATE will use DNSSec or just plain DNS
    pub dns_sec: bool,
    /// DNS server that queries will be made do by the chain registry
    pub dns_server: String,

    /// Synchronization tolerance whereby event duplication during connection phases
    /// and compaction efficiency are impacted. Greater tolerance will reduce the
    /// possibility of data lose on specific edge-cases while shorter tolerance will
    /// improve space and network efficiency. It is not recommended to select a value
    /// lower than a few seconds while increasing the value to days will impact performance.
    /// (default=30 seconds)
    pub sync_tolerance: Duration,

    /// Size of the local cache that stores redo log entries in memory
    #[cfg(feature = "enable_local_fs")]
    pub load_cache_size: usize,
    /// Number of seconds that redo log entries will remain in memory before
    /// they are evicted
    #[cfg(feature = "enable_local_fs")]
    pub load_cache_ttl: u64,

    /// Serialization format of the log files
    pub log_format: MessageFormat,
    /// Size of the buffer used by the chain-of-trust
    pub buffer_size_chain: usize,
    /// Timeout before an attempt to lock a data object fails
    pub lock_attempt_timeout: Duration,

    /// Flag that indicates if the type name should always be saved in the event log.
    /// Added the type-name consumes space but gives extra debug information
    pub record_type_name: bool,
}

impl Default
for ConfAte
{
    fn default() -> ConfAte {
        ConfAte {
            #[cfg(feature = "enable_local_fs")]
            log_path: None,
            dns_sec: false,
            dns_server: "8.8.8.8".to_string(),
            recovery_mode: RecoveryMode::ReadOnlyAsync,
            #[cfg(feature = "enable_local_fs")]
            backup_path: None,
            backup_mode: BackupMode::Full,
            compact_mode: CompactMode::Never,
            compact_bootstrap: false,
            compact_cleanup: false,
            sync_tolerance: Duration::from_secs(30),
            #[cfg(feature = "enable_ntp")]
            ntp_sync: true,
            #[cfg(feature = "enable_ntp")]
            ntp_pool: "pool.ntp.org".to_string(),
            #[cfg(feature = "enable_ntp")]
            ntp_port: 123,
            configured_for: ConfiguredFor::default(),
            #[cfg(feature = "enable_local_fs")]
            load_cache_size: 1000,
            #[cfg(feature = "enable_local_fs")]
            load_cache_ttl: 30,
            log_format: MessageFormat {
                meta: SerializationFormat::Bincode,
                data: SerializationFormat::Json,
            },
            buffer_size_chain: 1,
            lock_attempt_timeout: Duration::from_secs(20),
            record_type_name: false,
        }
    }
}