use wasm_bus::macros::*;

#[wasm_bus(format = "json")]
pub trait World {
    async fn hello(&self) -> String;
}