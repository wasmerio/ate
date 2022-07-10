# WASM Bus Macros

Helpful macros that make it easier to export and import APIs.

# Example API   

Below is a simple example of how to emit the helper classes and serialization objects for a specific API

```rust
use wasmer_bus::macros::*;

#[wasmer_bus(format = "json")]
pub trait Time {
    async fn sleep(&self, duration_ms: u128);
}
```

# Relative Objects

It is also possible to emit a series of sub-apis that carry with them some specific context

```rust
use serde::*;
use std::sync::Arc;
use wasmer_bus::macros::*;

#[wasmer_bus(format = "bincode")]
pub trait SocketBuilder {
    async fn connect(
        &self,
        url: String,
        state_change: impl Fn(SocketState),
        receive: impl Fn(Vec<u8>),
    ) -> Arc<dyn WebSocket>;
}

#[wasmer_bus(format = "bincode")]
pub trait WebSocket {
    async fn send(&self, data: Vec<u8>) -> SendResult;
}
```

# Testing

You can test your WASI program by uploading it to wapm.io and then heading over to the Wasmer Shell

https://wasmer.sh
