use tokio::sync::Mutex;
use async_trait::async_trait;
use wasm_bus_mio::api;
use wasm_bus_mio::prelude::*;
use wasm_bus_mio::api::MioResult;
use wasm_bus_mio::api::MioError;
use wasm_bus_mio::api::MioErrorKind;
use ate_mio::mio::Socket;

pub struct RawSocket
{
    socket: Mutex<Socket>,
}

impl RawSocket
{
    pub fn new(socket: Socket) -> RawSocket {
        RawSocket {
            socket: Mutex::new(socket)
        }
    }
}

#[async_trait]
impl api::RawSocketSimplified
for RawSocket {
    async fn send(&self, buf: Vec<u8>) -> MioResult<usize> {
        let socket = self.socket.lock().await;
        socket.send(buf)
            .await
            .map_err(|err| {
                let err: MioError = err.into();
                err
            })
    }

    async fn recv(&self, max: usize) -> MioResult<Vec<u8>> {
        let socket = self.socket.lock().await;
        socket.recv()
            .await
            .map_err(|err| {
                let err: MioError = err.into();
                err
            })
    }
}