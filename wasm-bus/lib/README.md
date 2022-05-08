# WASM General Purpose Bus

The WASM Genernal Purpose Bus implements a standard syscall interface
for WebAssembly libraries and executables to use to invoke and receive
calls between WASM modules and/or operating system functions.

Currently the WASM Bus is supported by the following runtimes:

- Tokera Terminal (https://tokera.sh/)

Functionality supported by this BUS includes

- Web Socket connections
- HTTP and HTTPS calls
- Spawning sub-processes
- Thread sleeping and timeouts
- Asynchronous Runtime Engine
- Client side invocation of WASM modules

# TODO

- Spawning threads
- Multi-threaded Runtime Engine
- Universal Logging
- Server side invocation of WASM modules

# Backend Implementations

In order to implment this BUS on your runtime one needs to chain to
the ABI exposed in this library and implement the functions.

For a reference implementation see below:

https://github.com/tokera-com/ate/tree/master/tokterm/src/bus

# Example API   

Below is a simple example of how to emit the helper classes and serialization objects for a specific API

```rust
use wasm_bus::macros::*;

#[wasm_bus(format = "json")]
pub trait Time {
    async fn sleep(&self, duration_ms: u128);
}
```

# Relative Objects

It is also possible to emit a series of sub-apis that carry with them some specific context

```rust
use serde::*;
use std::sync::Arc;
use wasm_bus::macros::*;

#[wasm_bus(format = "bincode")]
pub trait SocketBuilder {
    async fn connect(
        &self,
        url: String,
        state_change: impl Fn(SocketState),
        receive: impl Fn(Vec<u8>),
    ) -> Arc<dyn WebSocket>;
}

#[wasm_bus(format = "bincode")]
pub trait WebSocket {
    async fn send(&self, data: Vec<u8>) -> SendResult;
}
```

# Testing

You can test your WASI program by uploading it to wapm.io and then heading over to the Tokera Shell

https://tokera.sh
