#[allow(unused_imports)]
use tracing::{error, info, debug};
use error_chain::bail;

use crate::error::*;
use crate::meta::*;
use crate::lint::*;
use crate::plugin::*;
use crate::index::*;
use crate::session::*;
use crate::sink::*;
use crate::transform::*;
use crate::validator::*;
use crate::conf::*;
use crate::transaction::*;
use crate::event::EventHeader;

use std::{sync::Arc};
use std::time::Duration;
use std::time::UNIX_EPOCH;
use super::keeper::TimeKeeper;

#[derive(Debug, Clone)]
pub struct TimestampEnforcer {
    pub cursor: Duration,
    pub keeper: TimeKeeper,
}

impl TimestampEnforcer
{
    #[allow(dead_code)]
    pub async fn new(cfg: &ConfAte, tolerance_ms: u32) -> Result<TimestampEnforcer, TimeError>
    {
        let tolerance = Duration::from_millis(tolerance_ms as u64);
        Ok(
            TimestampEnforcer
            {
                cursor: tolerance,
                keeper: TimeKeeper::new(cfg, tolerance_ms).await?,
            }
        )
    }
}

impl EventMetadataLinter
for TimestampEnforcer
{
    fn clone_linter(&self) -> Box<dyn EventMetadataLinter> {
        Box::new(self.clone())
    }

    fn metadata_lint_event(&self, meta: &Metadata, _session: &'_ dyn AteSession, _trans_meta: &TransactionMetadata, _type_code: &str)-> Result<Vec<CoreMetadata>, LintError> {
        let mut ret = Vec::new();

        //println!("TIME: {} with offset of {} and ping of {}", self.current_timestamp()?, self.current_offset_ms(), self.current_ping_ms());

        if meta.get_timestamp().is_none() {
            ret.push(CoreMetadata::Timestamp(self.keeper.current_timestamp()?));
        }

        Ok(ret)
    }
}

impl EventSink
for TimestampEnforcer
{
    fn feed(&mut self, header: &EventHeader, _conversation: Option<&Arc<ConversationSession>>) -> Result<(), SinkError>
    {
        if let Some(time) = header.meta.get_timestamp() {
            let time = Duration::from_millis(time.time_since_epoch_ms);
            if time > self.cursor {
                self.cursor = time;
            }
        }
        Ok(())
    }   

    fn reset(&mut self) {
        self.cursor = self.keeper.tolerance.clone();
    }
}

impl EventIndexer
for TimestampEnforcer
{
    fn clone_indexer(&self) -> Box<dyn EventIndexer> {
        Box::new(self.clone())
    }
}

impl EventDataTransformer
for TimestampEnforcer
{
    fn clone_transformer(&self) -> Box<dyn EventDataTransformer> {
        Box::new(self.clone())
    }
}

impl EventValidator
for TimestampEnforcer
{
    fn clone_validator(&self) -> Box<dyn EventValidator> {
        Box::new(self.clone())
    }

    fn validate(&self, header: &EventHeader, _conversation: Option<&Arc<ConversationSession>>) -> Result<ValidationResult, ValidationError>
    {
        // If it does not have a timestamp then we can not accept it
        let time = match header.meta.get_timestamp() {
            Some(m) => m,
            None => {
                return match header.meta.needs_signature() {
                    true => {
                        debug!("rejected event due to missing timestamp");
                        Err(ValidationErrorKind::TrustError(TrustErrorKind::TimeError(TimeErrorKind::NoTimestamp)).into())
                    },
                    false => Ok(ValidationResult::Abstain)
                };
            },
        };

        // If time is not currently accurate then we can not properly validate
        if self.keeper.has_converged() == true
        {
            // Check its within the time range
            let timestamp = Duration::from_millis(time.time_since_epoch_ms);
            //let min_timestamp = self.cursor - self.tolerance;
            let max_timestamp = self.keeper.current_timestamp_as_duration()? + self.keeper.tolerance;
            
            if //timestamp < min_timestamp ||
            timestamp > max_timestamp
            {
                let cursor = UNIX_EPOCH + self.cursor;
                let timestamp = UNIX_EPOCH + timestamp;

                let cursor_str = chrono::DateTime::<chrono::Utc>::from(cursor).format("%Y-%m-%d %H:%M:%S.%f").to_string();
                let timestamp_str = chrono::DateTime::<chrono::Utc>::from(timestamp).format("%Y-%m-%d %H:%M:%S.%f").to_string();
                debug!("rejected event {:?} due to out-of-bounds timestamp ({} vs {})", header, cursor_str, timestamp_str);
                bail!(ValidationErrorKind::TrustError(TrustErrorKind::TimeError(TimeErrorKind::OutOfBounds(cursor, timestamp))));
            }
        }

        // All good
        Ok(ValidationResult::Abstain)
    }

    fn validator_name(&self) -> &str {
        "timestamp-validator"
    }
}

impl EventPlugin
for TimestampEnforcer
{
    fn clone_plugin(&self) -> Box<dyn EventPlugin> {
        Box::new(self.clone())
    }
}