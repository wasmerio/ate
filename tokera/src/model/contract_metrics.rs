use serde::*;
use chrono::DateTime;
use chrono::Utc;

/// Metrics are used to track provider services so that charges can
/// be made to the consumer at appropriate moments
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ContractMetrics
{
    /// What are these metrics related to
    pub related_to: String,

    /// Last time the flat rate was charged
    pub last_flat_rate: Option<DateTime<Utc>>,
    /// Last time the download charge was incurred
    pub last_per_download_terabyte: Option<DateTime<Utc>>,
    /// Last time the upload charge was incurred
    pub last_per_upload_terabyte: Option<DateTime<Utc>>,
    /// Last time the data storage charge was incurred
    pub last_per_stored_gigabyte: Option<DateTime<Utc>>,
    /// Last time the compute charge waas incurred
    pub last_per_compute_second: Option<DateTime<Utc>>,

    /// Current amount of compute usage accumilated since the last charge was made
    /// (measured in microseconds)
    pub current_accumilated_compute: u64,
    /// Current amount of download bandwidth accumilated since the last charge was made
    /// (measured in bytes)
    pub current_accumilated_download: u64,
    /// Current amount of upload bandwidth accumilated since the last charge was made
    /// (measured in bytes)
    pub current_accumilated_upload: u64,
    /// Current amount of storage capacity that is being consumed
    /// (measured in bytes)
    pub current_storage: u64,
}