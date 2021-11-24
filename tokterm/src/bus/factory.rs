use std::sync::Arc;
use wasm_bus::abi::CallError;

use super::*;

#[derive(Clone)]
pub struct BusFactory
{

}

impl BusFactory
{
    pub fn new() -> BusFactory {
        BusFactory {
        }
    }

    pub fn start(&self, _wapm: &str, _topic: &str) -> Arc<dyn Invokable>
    {
        ErrornousInvokable::new(CallError::InvalidWapm)
    }
}