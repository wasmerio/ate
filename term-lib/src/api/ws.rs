use async_trait::async_trait;

// This ABI implements a general purpose web socket
#[async_trait]
pub trait WebSocketAbi {
    fn set_onopen(&mut self, callback: Box<dyn FnMut()>);

    fn set_onclose(&mut self, callback: Box<dyn Fn() + Send + 'static>);

    fn set_onmessage(&mut self, callback: Box<dyn Fn(Vec<u8>) + Send + 'static>);

    #[cfg(feature = "async_ws")]
    async fn send(&mut self, data: Vec<u8>) -> Result<(), String>;

    #[cfg(not(feature = "async_ws"))]
    fn send(&mut self, data: Vec<u8>) -> Result<(), String>;
}
