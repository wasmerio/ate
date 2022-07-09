use std::collections::VecDeque;
use std::io;
use std::ops::DerefMut;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::Context;
use std::task::Poll;
use std::future::Future;
use std::io::Read;
use std::io::Write;
use bytes::Bytes;
use derivative::*;
use tokio::io::AsyncWrite;
use tokio::io::AsyncRead;
use tokio::io::ReadBuf;
use tokio::sync::mpsc;
use tokio::sync::watch;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::model::SendResult;
use crate::model::SocketState;
use wasm_bus::abi::*;

#[derive(Debug)]
pub struct WebSocket {
    pub(super) client: Arc<dyn crate::api::WebSocket>,
    pub(super) rx: mpsc::Receiver<Vec<u8>>,
    pub(super) state: watch::Receiver<SocketState>,
}

impl WebSocket {
    pub fn split(self) -> (SendHalf, RecvHalf) {
        (
            SendHalf {
                client: self.client,
                state: self.state,
                sending: Arc::new(Mutex::new(Default::default())),
            },
            RecvHalf {
                rx: self.rx,
                buffer: None,
            },
        )
    }
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct SendHalf {
    client: Arc<dyn crate::api::WebSocket>,
    state: watch::Receiver<SocketState>,
    #[derivative(Debug = "ignore")]
    sending: Arc<Mutex<VecDeque<Pin<Box<dyn Future<Output=Result<SendResult, BusError>> + Send + 'static>>>>>,
}

impl SendHalf {
    pub async fn wait_till_opened(&self) -> SocketState {
        let mut state = self.state.clone();
        while *state.borrow() == SocketState::Opening {
            if let Err(_) = state.changed().await {
                return SocketState::Closed;
            }
        }
        let ret = (*state.borrow()).clone();
        ret
    }

    pub async fn close(&self) -> io::Result<()> {
        Ok(())
    }

    pub async fn send(&mut self, data: Vec<u8>) -> io::Result<usize> {
        let state = self.wait_till_opened().await;
        if state != SocketState::Opened {
            return Err(io::Error::new(
                io::ErrorKind::ConnectionReset,
                format!("connection is not open (state={})", state).as_str(),
            ));
        }
        self.client
            .send(data)
            .await
            .map_err(|err| err.into_io_error())
            .map(|ret| match ret {
                SendResult::Success(a) => Ok(a),
                SendResult::Failed(err) => Err(io::Error::new(io::ErrorKind::Other, err)),
            })?
    }

    pub fn blocking_send(&mut self, data: Vec<u8>) -> io::Result<usize> {
        wasm_bus::task::block_on(self.send(data))
    }
}

impl AsyncWrite
for SendHalf
{
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>>
    {
        let buf_len = buf.len();
        let buf = buf.to_vec();
        let mut state = self.state.clone();
        let client = self.client.clone();
        let mut sending = self.sending.lock().unwrap();
        sending.push_back(Box::pin(async move {
            while *state.borrow() == SocketState::Opening {
                if let Err(_) = state.changed().await {
                    return Ok(SendResult::Failed("web socket is closed".to_string()));
                }
            }
            let state = *state.borrow();
            if state != SocketState::Opened {
                return Ok(SendResult::Failed(format!("connection is not open (state={})", state)));
            }
            client.send(buf).await
        }));
        Poll::Ready(Ok(buf_len))
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<Result<(), io::Error>>
    {
        let mut sending = self.sending.lock().unwrap();
        while let Some(mut fut) = sending.pop_front() {
            let fut_pinned = fut.as_mut();
            match fut_pinned.poll(cx) {
                Poll::Pending => {
                    sending.push_front(fut);
                    return Poll::Pending;
                }
                Poll::Ready(Err(err)) => {
                    return Poll::Ready(Err(err.into_io_error()));
                }
                Poll::Ready(Ok(_)) => {
                }
            }
        }
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>
    ) -> Poll<Result<(), io::Error>>
    {
        Poll::Ready(Ok(()))
    }
}

#[derive(Debug)]
pub struct RecvHalf {
    rx: mpsc::Receiver<Vec<u8>>,
    buffer: Option<Bytes>,
}

impl RecvHalf {
    pub async fn recv(&mut self) -> Option<Vec<u8>> {
        self.rx.recv().await
    }

    pub fn blocking_recv(&mut self) -> Option<Vec<u8>> {
        wasm_bus::task::block_on(self.rx.recv())
    }
}

impl AsyncRead
for RecvHalf
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if let Some(stream) = self.buffer.take() {
            if stream.len() <= buf.remaining() {
                buf.put_slice(&stream[..]);
            } else {
                let end = buf.remaining();
                buf.put_slice(&stream[..end]);
                self.buffer.replace(stream.slice(end..));
            }
            return Poll::Ready(Ok(()));
        }
        match self.rx.poll_recv(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(data)) => {
                if data.len() <= buf.remaining() {
                    buf.put_slice(&data[..]);
                } else {
                    let end = buf.remaining();
                    buf.put_slice(&data[..end]);
                    self.buffer.replace(Bytes::from(data).slice(end..));
                }
                Poll::Ready(Ok(()))
            },
            Poll::Ready(None) => {
                Poll::Ready(Err(io::Error::new(io::ErrorKind::NotConnected, "web socket connection has closed")))
            }
        }
    }
}
