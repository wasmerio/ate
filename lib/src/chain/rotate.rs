#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};

use crate::error::*;
use crate::trust::ChainHeader;
use crate::spec::*;

use super::*;

impl<'a> Chain
{
    #[cfg(feature = "enable_rotate")]
    pub async fn rotate(&'a self) -> Result<(), SerializationError>
    {
        // Start a new log file
        let mut single = self.single().await;

        // Build the header
        let header = ChainHeader {
            cut_off: single.inside_async.chain.timeline.end(),
        };
        let header_bytes = SerializationFormat::Json.serialize(&header)?;

        // Rotate the log
        single.inside_async.chain.redo.rotate(header_bytes).await?;
        Ok(())
    }
}