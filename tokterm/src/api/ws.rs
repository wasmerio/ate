use async_trait::async_trait;

// This ABI implements a general purpose web socket
#[async_trait]
pub trait WebSocketAbi
{
    fn set_onopen(&self, callback: Box<dyn FnMut()>);

    fn set_onclose(&self, callback: Box<dyn Fn()>);

    fn set_onmessage(&self, callback: Box<dyn Fn(Vec<u8>)>);

    fn send(&self, data: Vec<u8>) -> Result<(), String>;
}