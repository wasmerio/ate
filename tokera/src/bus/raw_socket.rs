use tokio::sync::Mutex;
use async_trait::async_trait;
use wasm_bus_mio::api;
use wasm_bus_mio::api::MioResult;
use wasm_bus_mio::api::MioError;
use ate_mio::mio::Socket;

#[derive(Debug)]
pub struct RawSocketServer
{
    socket: Mutex<Socket>,
}

impl RawSocketServer
{
    pub fn new(socket: Socket) -> RawSocketServer {
        RawSocketServer {
            socket: Mutex::new(socket)
        }
    }
}

#[async_trait]
impl api::RawSocketSimplified
for RawSocketServer {
    async fn send(&self, buf: Vec<u8>) -> MioResult<usize> {
        let socket = self.socket.lock().await;
        socket.send(buf)
            .await
            .map_err(|err| {
                let err: MioError = err.into();
                err
            })
    }

    async fn recv(&self, _max: usize) -> MioResult<Vec<u8>> {
        let mut socket = self.socket.lock().await;
        socket.recv()
            .await
            .map_err(|err| {
                let err: MioError = err.into();
                err
            })
    }
}