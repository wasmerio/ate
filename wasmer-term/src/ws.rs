use async_trait::async_trait;
use futures::stream::SplitSink;
use futures::stream::SplitStream;
use futures::SinkExt;
use futures_util::StreamExt;
use std::sync::Arc;
use std::sync::Mutex;
use wasmer_os::api::System;
use wasmer_os::api::SystemAbiExt;
use wasmer_os::api::WebSocketAbi;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

pub struct SysWebSocket {
    system: System,
    sink: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    stream: Option<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>,
    on_close: Arc<Mutex<Option<Box<dyn Fn() + Send + 'static>>>>,
}

impl SysWebSocket {
    pub async fn new(url: &str) -> Result<SysWebSocket, String> {
        let url = url::Url::parse(url)
            .map_err(|err| err.to_string())?;

        let (ws_stream, _) = connect_async(url).await
            .map_err(|err| format!("failed to connect - {}", err))?;
        let (sink, stream) = ws_stream.split();

        Ok(
            SysWebSocket {
                system: System::default(),
                sink,
                stream: Some(stream),
                on_close: Arc::new(Mutex::new(None)),
            }
        )
    }
}

#[async_trait]
impl WebSocketAbi for SysWebSocket {
    fn set_onopen(&mut self, mut callback: Box<dyn FnMut()>) {
        // We instantly notify that we are open
        callback();
    }

    fn set_onclose(&mut self, callback: Box<dyn Fn() + Send + 'static>) {
        let mut guard = self.on_close.lock().unwrap();
        guard.replace(callback);
    }

    fn set_onmessage(&mut self, callback: Box<dyn Fn(Vec<u8>) + Send + 'static>) {
        if let Some(mut stream) = self.stream.take() {
            let on_close = self.on_close.clone();
            self.system.fork_shared(move || async move {
                while let Some(msg) = stream.next().await {
                    match msg {
                        Ok(Message::Binary(msg)) => {
                            callback(msg);
                        }
                        a => {
                            debug!("received invalid msg: {:?}", a);
                        }
                    }
                }
                let on_close = on_close.lock().unwrap();
                if let Some(on_close) = on_close.as_ref() {
                    on_close();
                }
            });
        }
    }

    async fn send(&mut self, data: Vec<u8>) -> Result<(), String> {
        self.sink
            .send(Message::binary(data))
            .await
            .map_err(|err| err.to_string())?;
        Ok(())
    }
}
