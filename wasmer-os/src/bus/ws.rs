use crate::common::MAX_MPSC;
use derivative::Derivative;
use tokio::sync::broadcast;
use wasmer_bus_ws::model::SendResult;
use wasmer_vbus::BusDataFormat;
use wasmer_vbus::BusInvocationEvent;
use wasmer_vbus::InstantInvocation;
use wasmer_vbus::VirtualBusError;
use wasmer_vbus::VirtualBusInvocation;
use wasmer_vbus::VirtualBusInvokable;
use wasmer_vbus::VirtualBusInvoked;
use std::future::Future;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use tokio::select;
use tokio::sync::mpsc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasmer_bus::abi::SerializationFormat;
use wasmer_bus_ws::api;
use wasmer_bus_ws::model;

use super::*;
use crate::api::*;

pub fn web_socket(
    connect: api::SocketBuilderConnectRequest,
) -> WebSocket {
    let system = System::default();

    // Construct the channels
    let (tx_keepalive, mut rx_keepalive) = mpsc::channel(1);
    let (tx_state, rx_state) = mpsc::channel::<model::SocketState>(MAX_MPSC);
    let (tx_send, mut rx_send) = mpsc::channel::<Vec<u8>>(MAX_MPSC);
    let (tx_recv, rx_recv) = mpsc::channel::<Vec<u8>>(MAX_MPSC);

    // The web socket will be started in a background thread as it
    // is an asynchronous IO primative
    let sub_system = system.clone();
    system.spawn_dedicated_async(move || async move {

        // Open the web socket
        let (tx_state2, mut rx_state2) = broadcast::channel::<model::SocketState>(10);
        let mut ws_sys = match system.web_socket(connect.url.as_str()).await {
            Ok(a) => a,
            Err(err) => {
                debug!("failed to create web socket ({}): {}", connect.url, err);
                let _ = tx_state2.send(model::SocketState::Failed);
                return;
            }
        };

        {
            let tx_state2 = tx_state2.clone();  
            ws_sys.set_onopen(Box::new(move || {
                debug!("websocket set_onopen()");
                let _ = tx_state2.send(model::SocketState::Opened);
            }));
        }

        {
            let tx_state2 = tx_state2.clone();
            ws_sys.set_onclose(Box::new(move || {
                debug!("websocket set_onclose()");
                let _ = tx_state2.send(model::SocketState::Closed);
            }));
        }

        {
            ws_sys.set_onmessage(Box::new(move |data| {
                debug!("websocket recv {} bytes", data.len());
                sub_system.fire_and_forget(&tx_recv, data);
            }));
        }

        // Wait for the socket ot open or for something bad to happen
        loop {
            select! {
                _ = rx_keepalive.recv() => {
                    return;
                }
                state = rx_state2.recv() => {
                    match state {
                        Ok(state) => {
                            let _ = tx_state.send(state.clone()).await;
                            if state != model::SocketState::Opened {
                                return;
                            }
                            break;
                        }
                        Err(_) => {
                            return;
                        }
                    }
                }
            }
        }

        trace!("websocket is active");

        // The main loop does all the processing
        loop {
            select! {
                _ = rx_keepalive.recv() => {
                    trace!("websocket no longer alive");
                    break;
                }
                state = rx_state2.recv() => {
                    match state {
                        Ok(state) => {
                            trace!("websocket state change(val={})", state);
                            let _ = tx_state.send(state.clone()).await;
                            if state != model::SocketState::Opened {
                                break;
                            }
                        }
                        Err(_) => {
                            break;
                        }
                    }
                }
                request = rx_send.recv() => {
                    if let Some(data) = request {
                        let data_len = data.len();

                        #[cfg(feature="async_ws")]
                        let ret = ws_sys.send(data).await;
                        #[cfg(not(feature="async_ws"))]
                        let ret = ws_sys.send(data);

                        if let Err(err) = ret {
                            debug!("error sending message: {}", err);
                        } else {
                            trace!("websocket sent {} bytes", data_len);
                        }
                    } else {
                        trace!("websocket send-side closed");
                        break;
                    }
                }
            }
        }

        let _ = tx_state.send(model::SocketState::Closed).await;
    });

    // Return the WebSocket
    WebSocket {
        tx_keepalive,
        tx_send,
        rx_recv,
        rx_state,
    }
}

#[derive(Debug)]
pub struct WebSocket {
    #[allow(dead_code)]
    tx_keepalive: mpsc::Sender<()>,
    tx_send: mpsc::Sender<Vec<u8>>,
    rx_recv: mpsc::Receiver<Vec<u8>>,
    rx_state: mpsc::Receiver<model::SocketState>,
}

impl VirtualBusInvocation
for WebSocket
{
    fn poll_event(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<BusInvocationEvent> {
        loop {
            match self.rx_recv.poll_recv(cx) {
                Poll::Ready(Some(data)) => {
                    let data = api::SocketBuilderConnectReceiveCallback(data);
                    return Poll::Ready(BusInvocationEvent::Callback {
                        topic_hash: type_name_hash::<api::SocketBuilderConnectReceiveCallback>(),
                        format: BusDataFormat::Bincode,
                        data: match SerializationFormat::Bincode.serialize(data) {
                            Ok(d) => d,
                            Err(err) => {
                                debug!("failed to serialize web socket received data");
                                return Poll::Ready(BusInvocationEvent::Fault { fault: conv_error_back(err) });
                            }
                        }
                    });
                },
                Poll::Ready(None) => {
                    return Poll::Ready(BusInvocationEvent::Fault { fault: VirtualBusError::Aborted });
                }
                Poll::Pending => { },
            }
            let mut rx = Pin::new(&mut self.rx_state);
            match rx.poll_recv(cx) {
                Poll::Ready(Some(state)) => {
                    match state {
                        model::SocketState::Opened => {
                            debug!("confirmed websocket successfully opened");
                            let data = api::SocketBuilderConnectStateChangeCallback(state);
                            return Poll::Ready(BusInvocationEvent::Callback {
                                topic_hash: type_name_hash::<api::SocketBuilderConnectStateChangeCallback>(),
                                format: BusDataFormat::Bincode,
                                data: match SerializationFormat::Bincode.serialize(data) {
                                    Ok(d) => d,
                                    Err(err) => {
                                        debug!("failed to serialize web socket received data");
                                        return Poll::Ready(BusInvocationEvent::Fault { fault: conv_error_back(err) });
                                    }
                                }
                            });
                        },
                        _ => {
                            debug!("confirmed websocket closed");
                            return Poll::Ready(BusInvocationEvent::Fault { fault: VirtualBusError::Aborted });
                        }
                    }
                },                
                Poll::Ready(None) => {
                    debug!("confirmed websocket closed");
                    return Poll::Ready(BusInvocationEvent::Fault { fault: VirtualBusError::Aborted });
                },
                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }
    }
}

impl VirtualBusInvokable
for WebSocket {
    /// Invokes a service within this instance
    fn invoke(
        &self,
        topic_hash: u128,
        format: BusDataFormat,
        buf: Vec<u8>,
    ) -> Box<dyn VirtualBusInvoked> {
        if topic_hash == type_name_hash::<api::WebSocketSendRequest>() {
            debug!("websocket send {} bytes", buf.len());
            let data = match decode_request::<api::WebSocketSendRequest>(
                format,
                buf,
            ) {
                Ok(a) => a.data,
                Err(err) => {
                    return Box::new(InstantInvocation::fault(conv_error_back(err)));
                }
            };
            let data_len = data.len();

            let tx = self.tx_send.clone();
            let fut = async move {
                tx.send(data).await
            };
            Box::new(DelayedSend {
                data_len,
                fut: Box::pin(fut)
            })
        } else {
            debug!("websocket invalid topic (hash={})", topic_hash);
            Box::new(InstantInvocation::fault(VirtualBusError::InvalidTopic))
        }
    }
}

impl Drop
for WebSocket
{
    fn drop(&mut self) {
        trace!("websocket dropped");
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
struct DelayedSend
{
    data_len: usize,
    #[derivative(Debug = "ignore")]
    fut: Pin<Box<dyn Future<Output = Result<(), mpsc::error::SendError<Vec<u8>>>>>>
}

impl VirtualBusInvoked
for DelayedSend
{
    fn poll_invoked(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Box<dyn VirtualBusInvocation + Sync>, VirtualBusError>>
    {
        let fut = self.fut.as_mut();
        match fut.poll(cx) {
            Poll::Ready(Ok(())) => {
                Poll::Ready(Ok(Box::new(encode_instant_response(BusDataFormat::Bincode,
                    &SendResult::Success(self.data_len)))))
            },
            Poll::Ready(Err(err)) => {
                Poll::Ready(Ok(Box::new(encode_instant_response(BusDataFormat::Bincode,
                    &SendResult::Failed(err.to_string())))))
            },
            Poll::Pending => Poll::Pending
        }
    }
}
