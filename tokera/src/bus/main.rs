use crate::error::*;
use crate::opt::OptsBus;
use wasm_bus::backend::fuse::*;
use wasm_bus::task::*;

pub async fn main_opts_bus(
    _opts: OptsBus,
    _token_path: String,
    _auth_url: url::Url,
) -> Result<(), BusError> {
    // Register all the functions
    ListenerBuilder::new(move |_mount: Mount| async move {
        panic!("we made it!");
    });

    // Enter a polling loop
    serve();
    Ok(())
}
