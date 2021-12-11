use crate::error::*;
use crate::opt::OptsBus;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::backend::fuse::*;
use wasm_bus::task;

pub async fn main_opts_bus(
    _opts: OptsBus,
    _token_path: String,
    _auth_url: url::Url,
) -> Result<(), BusError> {
    info!("wasm bus initializing");

    // Register all the functions
    task::ListenerBuilder::new(move |_mount: Mount| async move {
        info!("we made it! - MOUNT");
    })
    .add(task::ListenerBuilder::new(move |_meta: ReadSymlinkMetadata| async move {
        info!("we made it! - META");
    }))
    .listen();

    // Enter a polling loop
    task::serve();
    Ok(())
}
