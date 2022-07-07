use std::future::Future;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use derivative::Derivative;
use tokio::sync::mpsc;
use wasm_bus::abi::SerializationFormat;
use wasmer_vbus::BusDataFormat;
use wasmer_vbus::BusInvocationEvent;
use wasmer_vbus::InstantInvocation;
use wasmer_vbus::VirtualBusError;
use wasmer_vbus::VirtualBusInvocation;
use wasmer_vbus::VirtualBusInvokable;
use wasmer_vbus::VirtualBusInvoked;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct AsyncResult<T> {
    #[derivative(Debug = "ignore")]
    pub rx: mpsc::Receiver<T>,
    pub(crate) format: SerializationFormat,
}

impl<T> AsyncResult<T> {
    pub fn new(format: SerializationFormat, rx: mpsc::Receiver<T>) -> Self {
        Self { rx, format }
    }

    pub fn block_on(mut self) -> Option<T> {
        self.rx.blocking_recv()
    }
}

impl<T> Future for AsyncResult<T> {
    type Output = Option<T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.rx.poll_recv(cx)
    }
}

impl<T> VirtualBusInvocation
for AsyncResult<T>
where T: Send + 'static,
      T: serde::ser::Serialize
{
    fn poll_event(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<BusInvocationEvent> {
        match self.rx.poll_recv(cx) {
            Poll::Ready(Some(data)) => {
                Poll::Ready(BusInvocationEvent::Response {
                    format: crate::bus::conv_format_back(self.format),
                    data: match self.format.serialize(data) {
                        Ok(d) => d,
                        Err(err) => {
                            return Poll::Ready(BusInvocationEvent::Fault { fault: crate::bus::conv_error_back(err) });
                        }
                    }
                })
            },
            Poll::Ready(None) => {
                Poll::Ready(BusInvocationEvent::Fault { fault: VirtualBusError::Aborted })
            },
            Poll::Pending => Poll::Pending
        }
    }
}

impl<T> VirtualBusInvokable
for AsyncResult<T>
where T: Send + 'static
{
    fn invoke(
        &self,
        _topic_hash: u128,
        _format: BusDataFormat,
        _buf: Vec<u8>,
    ) -> Box<dyn VirtualBusInvoked> {
        Box::new(InstantInvocation::fault(VirtualBusError::InvalidTopic))
    }
}
