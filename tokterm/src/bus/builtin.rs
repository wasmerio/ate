use std::any::type_name;
use std::sync::Arc;
use wasm_bus::backend;

use super::*;

pub fn builtin(wapm: &str, topic: &str) -> Option<Arc<dyn Invokable>> {
    match (wapm, topic) {
        ("os", topic) if topic == type_name::<backend::ws::Connect>() => {
            let invoke = CallbackInvokable::new(move |request| async move {
                let ws = WebSocket::new(request);
                ws.run().await
            });
            Some(Arc::new(invoke))
        }
        _ => None,
    }
}
