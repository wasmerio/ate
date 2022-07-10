use wasmer_bus::macros::*;

#[wasmer_bus(format = "json")]
pub trait World {
    async fn hello(&self) -> String;
}