use std::sync::Arc;
use wasm_bus::macros::*;

use crate::model::SendResult;
use crate::model::SocketState;

#[wasm_bus(format = "bincode")]
pub trait SocketBuilder {
    async fn connect(
        &self,
        url: String,
        state_change: impl Fn(SocketState),
        receive: impl Fn(Vec<u8>),
    ) -> Arc<dyn WebSocket>;
}

#[wasm_bus(format = "bincode")]
pub trait WebSocket {
    async fn send(&self, data: Vec<u8>) -> SendResult;
}

/*
#[derive(Debug, Clone, serde :: Serialize, serde :: Deserialize)]
pub struct SocketBuilderConnectStateChangeCallback(pub SocketState);
#[derive(Debug, Clone, serde :: Serialize, serde :: Deserialize)]
pub struct SocketBuilderConnectReceiveCallback(pub Vec<u8>);
#[derive(Debug, Clone, serde :: Serialize, serde :: Deserialize)]
pub struct SocketBuilderConnectRequest {
    pub url: String,
}
#[wasm_bus::async_trait]
pub trait SocketBuilder
where
    Self: std::fmt::Debug + Send + Sync,
{
    async fn connect(
        &self,
        url: String,
        state_change: Box<dyn Fn(SocketState) + Send + Sync + 'static>,
        receive: Box<dyn Fn(Vec<u8>) + Send + Sync + 'static>,
    ) -> std::result::Result<std::sync::Arc<dyn WebSocket>, wasm_bus::abi::BusError>;
    fn blocking_connect(
        &self,
        url: String,
        state_change: Box<dyn Fn(SocketState) + Send + Sync + 'static>,
        receive: Box<dyn Fn(Vec<u8>) + Send + Sync + 'static>,
    ) -> std::result::Result<std::sync::Arc<dyn WebSocket>, wasm_bus::abi::BusError>;
    fn as_client(&self) -> Option<SocketBuilderClient>;
}
#[wasm_bus::async_trait]
pub trait SocketBuilderSimplified
where
    Self: std::fmt::Debug + Send + Sync,
{
    async fn connect(
        &self,
        url: String,
        state_change: Box<dyn Fn(SocketState) + Send + Sync + 'static>,
        receive: Box<dyn Fn(Vec<u8>) + Send + Sync + 'static>,
    ) -> std::result::Result<std::sync::Arc<dyn WebSocket>, wasm_bus::abi::BusError>;
}
#[wasm_bus::async_trait]
impl<T> SocketBuilder for T
where
    T: SocketBuilderSimplified,
{
    async fn connect(
        &self,
        url: String,
        state_change: Box<dyn Fn(SocketState) + Send + Sync + 'static>,
        receive: Box<dyn Fn(Vec<u8>) + Send + Sync + 'static>,
    ) -> std::result::Result<std::sync::Arc<dyn WebSocket>, wasm_bus::abi::BusError> {
        SocketBuilderSimplified::connect(self, url, state_change, receive).await
    }
    fn blocking_connect(
        &self,
        url: String,
        state_change: Box<dyn Fn(SocketState) + Send + Sync + 'static>,
        receive: Box<dyn Fn(Vec<u8>) + Send + Sync + 'static>,
    ) -> std::result::Result<std::sync::Arc<dyn WebSocket>, wasm_bus::abi::BusError> {
        wasm_bus::task::block_on(SocketBuilderSimplified::connect(
            self,
            url,
            state_change,
            receive,
        ))
    }
    fn as_client(&self) -> Option<SocketBuilderClient> {
        None
    }
}
#[derive(Debug, Clone)]
pub struct SocketBuilderService {}
impl SocketBuilderService {
    #[allow(dead_code)]
    pub(crate) fn attach(
        wasm_me: std::sync::Arc<dyn SocketBuilder>,
        call_handle: wasm_bus::abi::CallSmartHandle,
    ) {
        {
            let wasm_me = wasm_me.clone();
            let call_handle = call_handle.clone();
            wasm_bus::task::respond_to(
                call_handle,
                wasm_bus::abi::SerializationFormat::Bincode,
                #[allow(unused_variables)]
                move |wasm_handle: wasm_bus::abi::CallHandle,
                      wasm_req: SocketBuilderConnectRequest| {
                    let wasm_me = wasm_me.clone();
                    let wasm_handle = wasm_bus::abi::CallSmartHandle::new(wasm_handle);
                    let url = wasm_req.url;
                    async move {
                        let state_change = {
                            let wasm_handle = wasm_handle.clone();
                            Box::new(move |response: SocketState| {
                                let response = SocketBuilderConnectStateChangeCallback(response);
                                let _ = wasm_bus::abi::subcall(
                                    wasm_handle.clone(),
                                    wasm_bus::abi::SerializationFormat::Bincode,
                                    response,
                                )
                                .invoke();
                            })
                        };
                        let receive = {
                            let wasm_handle = wasm_handle.clone();
                            Box::new(move |response: Vec<u8>| {
                                let response = SocketBuilderConnectReceiveCallback(response);
                                let _ = wasm_bus::abi::subcall(
                                    wasm_handle.clone(),
                                    wasm_bus::abi::SerializationFormat::Bincode,
                                    response,
                                )
                                .invoke();
                            })
                        };
                        let svc = wasm_me.connect(url, state_change, receive).await?;
                        WebSocketService::attach(svc, wasm_handle);
                        Ok(())
                    }
                },
                true,
            );
        }
    }
    pub fn listen(wasm_me: std::sync::Arc<dyn SocketBuilder>) {
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::listen(
                wasm_bus::abi::SerializationFormat::Bincode,
                #[allow(unused_variables)]
                move |wasm_handle: wasm_bus::abi::CallHandle,
                      wasm_req: SocketBuilderConnectRequest| {
                    let wasm_me = wasm_me.clone();
                    let wasm_handle = wasm_bus::abi::CallSmartHandle::new(wasm_handle);
                    let url = wasm_req.url;
                    async move {
                        let state_change = {
                            let wasm_handle = wasm_handle.clone();
                            Box::new(move |response: SocketState| {
                                let response = SocketBuilderConnectStateChangeCallback(response);
                                let _ = wasm_bus::abi::subcall(
                                    wasm_handle.clone(),
                                    wasm_bus::abi::SerializationFormat::Bincode,
                                    response,
                                )
                                .invoke();
                            })
                        };
                        let receive = {
                            let wasm_handle = wasm_handle.clone();
                            Box::new(move |response: Vec<u8>| {
                                let response = SocketBuilderConnectReceiveCallback(response);
                                let _ = wasm_bus::abi::subcall(
                                    wasm_handle.clone(),
                                    wasm_bus::abi::SerializationFormat::Bincode,
                                    response,
                                )
                                .invoke();
                            })
                        };
                        let svc = wasm_me.connect(url, state_change, receive).await?;
                        WebSocketService::attach(svc, wasm_handle);
                        Ok(())
                    }
                },
            );
        }
    }
    pub fn serve() {
        wasm_bus::task::serve();
    }
}
#[derive(Debug, Clone)]
pub struct SocketBuilderClient {
    ctx: wasm_bus::abi::CallContext,
    task: Option<wasm_bus::abi::Call>,
    join: Option<wasm_bus::abi::CallJoin<()>>,
}
impl SocketBuilderClient {
    pub fn new(wapm: &str) -> Self {
        Self {
            ctx: wasm_bus::abi::CallContext::NewBusCall {
                wapm: wapm.to_string().into(),
                instance: None,
            },
            task: None,
            join: None,
        }
    }
    pub fn new_with_instance(wapm: &str, instance: &str, access_token: &str) -> Self {
        Self {
            ctx: wasm_bus::abi::CallContext::NewBusCall {
                wapm: wapm.to_string().into(),
                instance: Some(wasm_bus::abi::CallInstance::new(instance, access_token)),
            },
            task: None,
            join: None,
        }
    }
    pub fn attach(handle: wasm_bus::abi::CallSmartHandle) -> Self {
        Self {
            ctx: wasm_bus::abi::CallContext::SubCall { parent: handle },
            task: None,
            join: None,
        }
    }
    pub fn wait(self) -> Result<(), wasm_bus::abi::BusError> {
        if let Some(join) = self.join {
            join.wait()?;
        }
        if let Some(task) = self.task {
            task.join()?.wait()?;
        }
        Ok(())
    }
    pub fn try_wait(&mut self) -> Result<Option<()>, wasm_bus::abi::BusError> {
        if let Some(task) = self.task.take() {
            self.join.replace(task.join()?);
        }
        if let Some(join) = self.join.as_mut() {
            join.try_wait()
        } else {
            Ok(None)
        }
    }
    pub async fn connect(
        &self,
        url: String,
        state_change: Box<dyn Fn(SocketState) + Send + Sync + 'static>,
        receive: Box<dyn Fn(Vec<u8>) + Send + Sync + 'static>,
    ) -> std::result::Result<std::sync::Arc<dyn WebSocket>, wasm_bus::abi::BusError> {
        let request = SocketBuilderConnectRequest { url };
        let handle = wasm_bus::abi::call(
            self.ctx.clone(),
            wasm_bus::abi::SerializationFormat::Bincode,
            request,
        )
        .callback(move |req: SocketBuilderConnectStateChangeCallback| state_change(req.0))
        .callback(move |req: SocketBuilderConnectReceiveCallback| receive(req.0))
        .detach()?;
        Ok(Arc::new(WebSocketClient::attach(handle)))
    }
    pub fn blocking_connect(
        &self,
        url: String,
        state_change: Box<dyn Fn(SocketState) + Send + Sync + 'static>,
        receive: Box<dyn Fn(Vec<u8>) + Send + Sync + 'static>,
    ) -> std::result::Result<std::sync::Arc<dyn WebSocket>, wasm_bus::abi::BusError> {
        wasm_bus::task::block_on(self.connect(url, state_change, receive))
    }
}
impl std::future::Future for SocketBuilderClient {
    type Output = Result<(), wasm_bus::abi::BusError>;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if let Some(task) = self.task.take() {
            self.join.replace(task.join()?);
        }
        if let Some(join) = self.join.as_mut() {
            let join = std::pin::Pin::new(join);
            return join.poll(cx);
        } else {
            std::task::Poll::Ready(Ok(()))
        }
    }
}
#[wasm_bus::async_trait]
impl SocketBuilder for SocketBuilderClient {
    async fn connect(
        &self,
        url: String,
        state_change: Box<dyn Fn(SocketState) + Send + Sync + 'static>,
        receive: Box<dyn Fn(Vec<u8>) + Send + Sync + 'static>,
    ) -> std::result::Result<std::sync::Arc<dyn WebSocket>, wasm_bus::abi::BusError> {
        SocketBuilderClient::connect(self, url, state_change, receive).await
    }
    fn blocking_connect(
        &self,
        url: String,
        state_change: Box<dyn Fn(SocketState) + Send + Sync + 'static>,
        receive: Box<dyn Fn(Vec<u8>) + Send + Sync + 'static>,
    ) -> std::result::Result<std::sync::Arc<dyn WebSocket>, wasm_bus::abi::BusError> {
        SocketBuilderClient::blocking_connect(self, url, state_change, receive)
    }
    fn as_client(&self) -> Option<SocketBuilderClient> {
        Some(self.clone())
    }
}

#[derive(Debug, Clone, serde :: Serialize, serde :: Deserialize)]
pub struct WebSocketSendRequest {
    pub data: Vec<u8>,
}
#[wasm_bus::async_trait]
pub trait WebSocket
where
    Self: std::fmt::Debug + Send + Sync,
{
    async fn send(&self, data: Vec<u8>)
        -> std::result::Result<SendResult, wasm_bus::abi::BusError>;
    fn blocking_send(
        &self,
        data: Vec<u8>,
    ) -> std::result::Result<SendResult, wasm_bus::abi::BusError>;
    fn as_client(&self) -> Option<WebSocketClient>;
}
#[wasm_bus::async_trait]
pub trait WebSocketSimplified
where
    Self: std::fmt::Debug + Send + Sync,
{
    async fn send(&self, data: Vec<u8>) -> SendResult;
}
#[wasm_bus::async_trait]
impl<T> WebSocket for T
where
    T: WebSocketSimplified,
{
    async fn send(
        &self,
        data: Vec<u8>,
    ) -> std::result::Result<SendResult, wasm_bus::abi::BusError> {
        Ok(WebSocketSimplified::send(self, data).await)
    }
    fn blocking_send(
        &self,
        data: Vec<u8>,
    ) -> std::result::Result<SendResult, wasm_bus::abi::BusError> {
        Ok(wasm_bus::task::block_on(WebSocketSimplified::send(
            self, data,
        )))
    }
    fn as_client(&self) -> Option<WebSocketClient> {
        None
    }
}
#[derive(Debug, Clone)]
pub struct WebSocketService {}
impl WebSocketService {
    #[allow(dead_code)]
    pub(crate) fn attach(
        wasm_me: std::sync::Arc<dyn WebSocket>,
        call_handle: wasm_bus::abi::CallSmartHandle,
    ) {
        {
            let wasm_me = wasm_me.clone();
            let call_handle = call_handle.clone();
            wasm_bus::task::respond_to(
                call_handle,
                wasm_bus::abi::SerializationFormat::Bincode,
                #[allow(unused_variables)]
                move |wasm_handle: wasm_bus::abi::CallHandle, wasm_req: WebSocketSendRequest| {
                    let wasm_me = wasm_me.clone();
                    let wasm_handle = wasm_bus::abi::CallSmartHandle::new(wasm_handle);
                    let data = wasm_req.data;
                    async move { wasm_me.send(data).await }
                },
            );
        }
    }
    pub fn listen(wasm_me: std::sync::Arc<dyn WebSocket>) {
        {
            let wasm_me = wasm_me.clone();
            wasm_bus::task::listen(
                wasm_bus::abi::SerializationFormat::Bincode,
                #[allow(unused_variables)]
                move |_wasm_handle: wasm_bus::abi::CallHandle, wasm_req: WebSocketSendRequest| {
                    let wasm_me = wasm_me.clone();
                    let data = wasm_req.data;
                    async move { wasm_me.send(data).await }
                },
            );
        }
    }
    pub fn serve() {
        wasm_bus::task::serve();
    }
}
#[derive(Debug, Clone)]
pub struct WebSocketClient {
    ctx: wasm_bus::abi::CallContext,
    task: Option<wasm_bus::abi::Call>,
    join: Option<wasm_bus::abi::CallJoin<()>>,
}
impl WebSocketClient {
    pub fn new(wapm: &str) -> Self {
        Self {
            ctx: wasm_bus::abi::CallContext::NewBusCall {
                wapm: wapm.to_string().into(),
                instance: None,
            },
            task: None,
            join: None,
        }
    }
    pub fn new_with_instance(wapm: &str, instance: &str, access_token: &str) -> Self {
        Self {
            ctx: wasm_bus::abi::CallContext::NewBusCall {
                wapm: wapm.to_string().into(),
                instance: Some(wasm_bus::abi::CallInstance::new(instance, access_token)),
            },
            task: None,
            join: None,
        }
    }
    pub fn attach(handle: wasm_bus::abi::CallSmartHandle) -> Self {
        Self {
            ctx: wasm_bus::abi::CallContext::SubCall { parent: handle },
            task: None,
            join: None,
        }
    }
    pub fn wait(self) -> Result<(), wasm_bus::abi::BusError> {
        if let Some(join) = self.join {
            join.wait()?;
        }
        if let Some(task) = self.task {
            task.join()?.wait()?;
        }
        Ok(())
    }
    pub fn try_wait(&mut self) -> Result<Option<()>, wasm_bus::abi::BusError> {
        if let Some(task) = self.task.take() {
            self.join.replace(task.join()?);
        }
        if let Some(join) = self.join.as_mut() {
            join.try_wait()
        } else {
            Ok(None)
        }
    }
    pub async fn send(
        &self,
        data: Vec<u8>,
    ) -> std::result::Result<SendResult, wasm_bus::abi::BusError> {
        let request = WebSocketSendRequest { data };
        wasm_bus::abi::call(
            self.ctx.clone(),
            wasm_bus::abi::SerializationFormat::Bincode,
            request,
        )
        .invoke()
        .join()?
        .await
    }
    pub fn blocking_send(
        &self,
        data: Vec<u8>,
    ) -> std::result::Result<SendResult, wasm_bus::abi::BusError> {
        wasm_bus::task::block_on(self.send(data))
    }
}
impl std::future::Future for WebSocketClient {
    type Output = Result<(), wasm_bus::abi::BusError>;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if let Some(task) = self.task.take() {
            self.join.replace(task.join()?);
        }
        if let Some(join) = self.join.as_mut() {
            let join = std::pin::Pin::new(join);
            return join.poll(cx);
        } else {
            std::task::Poll::Ready(Ok(()))
        }
    }
}
#[wasm_bus::async_trait]
impl WebSocket for WebSocketClient {
    async fn send(
        &self,
        data: Vec<u8>,
    ) -> std::result::Result<SendResult, wasm_bus::abi::BusError> {
        WebSocketClient::send(self, data).await
    }
    fn blocking_send(
        &self,
        data: Vec<u8>,
    ) -> std::result::Result<SendResult, wasm_bus::abi::BusError> {
        WebSocketClient::blocking_send(self, data)
    }
    fn as_client(&self) -> Option<WebSocketClient> {
        Some(self.clone())
    }
}
*/
