use crate::error::*;
use crate::opt::OptsBus;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::backend::fuse::*;
use wasm_bus::task::*;

pub async fn main_opts_bus(
    _opts: OptsBus,
    _token_path: String,
    _auth_url: url::Url,
) -> Result<(), BusError> {
    // Initialize the logging and panic hook
    #[cfg(target_os = "wasi")]
    init_wasi();

    // Register all the functions
    ListenerBuilder::new(move |_mount: Mount| async move {
        info!("we made it!");
    });

    // Enter a polling loop
    serve();
    Ok(())
}
