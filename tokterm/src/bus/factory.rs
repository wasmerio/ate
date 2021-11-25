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

    pub fn start(&self, parent: Option<u32>, wapm: &str, topic: &str) -> Arc<dyn Invokable>
    {
        if let Some(invoker) = super::builtin::builtin(parent, wapm, topic) {
            return invoker;
        }

        ErrornousInvokable::new(CallError::InvalidWapm)
    }
}