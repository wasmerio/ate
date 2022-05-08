use std::sync::Arc;
use async_trait::async_trait;
use wasm_bus_hello::*;

#[derive(Debug, Default)]
struct HelloService
{
}

#[async_trait]
impl WorldSimplified
for HelloService
{
    async fn hello(&self) -> String {
        "hello".to_string()
    }
}

fn main() {
    WorldService::listen(Arc::new(HelloService::default()));
    WorldService::serve();
}