#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use crate::error::*;
use crate::spec::*;
use crate::trust::ChainHeader;

use super::*;

impl<'a> Chain {
    #[cfg(feature = "enable_rotate")]
    pub async fn rotate(&'a self) -> Result<(), SerializationError> {
        let delayed_operations = {
            // Switch to single-user mode while we make the rotation
            // of the log file - this will effectively freeze all IO
            // operations on this datachain while the rotate happens
            let mut single = self.single().await;

            // Build the header
            let header = ChainHeader {
                cut_off: single.inside_async.chain.timeline.end(),
            };
            let header_bytes = SerializationFormat::Json.serialize(&header)?;

            // Rotate the log
            single.inside_async.chain.redo.rotate(header_bytes).await?;

            // If there are any backups then we should run these on any
            // of the archive files that are now in a state where backup
            // can take place
            single.inside_async.chain.redo.backup(false)?
        };

        delayed_operations.await?;
        Ok(())
    }
}
