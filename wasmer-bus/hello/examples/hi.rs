use std::sync::Arc;
use async_trait::async_trait;
use wasmer_bus_hello::*;

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

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    WorldService::listen(Arc::new(HelloService::default()));
    WorldService::serve().await;
}