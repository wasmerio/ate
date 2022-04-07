use wasm_bus::macros::*;

#[wasm_bus(format = "json")]
pub trait Time {
    async fn sleep(&self, duration_ms: u128);
}
