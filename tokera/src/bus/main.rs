use crate::error::*;
use crate::opt::OptsBus;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::backend::fuse::*;
use wasm_bus::prelude::*;

pub async fn main_opts_bus(
    _opts: OptsBus,
    _token_path: String,
    _auth_url: url::Url,
) -> Result<(), BusError> {
    info!("wasm bus initializing");

    // Register all the functions
    listen(move |handle: CallHandle, _mount: Mount| async move {
        info!("we made it! - MOUNT");

        respond_to(
            handle,
            move |_handle, _meta: ReadSymlinkMetadata| async move {
                info!("we made it! - META");
            },
        );
    });

    // Enter a polling loop
    serve();
    Ok(())
}
