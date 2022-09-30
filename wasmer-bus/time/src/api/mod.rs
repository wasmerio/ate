use wasmer_bus::macros::*;

#[wasmer_bus(format = "json")]
pub trait Time {
    async fn sleep(&self, duration_ms: u128);
}
