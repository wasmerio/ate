use std::sync::Arc;
use std::any::type_name;
use wasm_bus::backend;

use super::*;

pub fn builtin(_parent: Option<u32>, wapm: &str, topic: &str) -> Option<Arc<dyn Invokable>> {
    match (wapm, topic) {
        ("os", topic) if topic == type_name::<backend::ws::Connect>() => {
            Some(CallbackInvokable::new(process_ws_connect))
        }
        _ => None
    }
}

async fn process_ws_connect(_request: backend::ws::Connect) -> () {

}