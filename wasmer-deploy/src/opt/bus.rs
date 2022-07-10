use clap::Parser;

#[allow(dead_code)]
#[derive(Parser)]
pub struct OptsBus {
    /// URL where the data is remotely stored on a distributed commit log (e.g. wss://wasmer.sh/db).
    #[clap(short, long)]
    pub remote: Option<url::Url>,
    /// Determines how the file-system will react while it is nominal and when it is
    /// recovering from a communication failure (valid options are 'async', 'readonly-async',
    /// 'readonly-sync' or 'sync')
    #[clap(long, default_value = "readonly-async")]
    pub recovery_mode: ate::mesh::RecoveryMode,
    /// Configure the log file for <raw>, <barebone>, <speed>, <compatibility>, <balanced> or <security>
    #[clap(long, default_value = "speed")]
    pub configured_for: ate::conf::ConfiguredFor,
    /// Format of the metadata in the log file as <bincode>, <json> or <mpack>
    #[clap(long, default_value = "bincode")]
    pub meta_format: ate::spec::SerializationFormat,
    /// Format of the data in the log file as <bincode>, <json> or <mpack>
    #[clap(long, default_value = "bincode")]
    pub data_format: ate::spec::SerializationFormat,
    /// Mode that the compaction will run under (valid modes are 'never', 'modified', 'timer', 'factor', 'size', 'factor-or-timer', 'size-or-timer')
    #[clap(long, default_value = "factor-or-timer")]
    pub compact_mode: ate::compact::CompactMode,
    /// Time in seconds between compactions of the log file (default: 1 hour) - this argument is ignored if you select a compact_mode that has no timer
    #[clap(long, default_value = "3600")]
    pub compact_timer: u64,
    /// Factor growth in the log file which will trigger compaction - this argument is ignored if you select a compact_mode that has no growth trigger
    #[clap(long, default_value = "0.4")]
    pub compact_threshold_factor: f32,
    /// Size of growth in bytes in the log file which will trigger compaction (default: 100MB) - this argument is ignored if you select a compact_mode that has no growth trigger
    #[clap(long, default_value = "104857600")]
    pub compact_threshold_size: u64,
}
