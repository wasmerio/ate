#[allow(unused_imports)]
use tracing::{info, error, warn, debug};
use async_trait::async_trait;
use serde::{Serialize, de::DeserializeOwned};
use bytes::Bytes;
use std::future::Future;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::spec::SerializationFormat;
use crate::error::*;

#[async_trait]
pub trait ServiceInvoker
where Self: Send + Sync
{
    async fn invoke(&self, request: Bytes) -> Result<Result<Bytes, Bytes>, SerializationError>;

    fn data_format(&self) -> SerializationFormat;

    fn request_type_name(&self) -> String;

    fn response_type_name(&self) -> String;

    fn error_type_name(&self) -> String;
}

pub struct ServiceHandler<CTX, REQ, RES, ERR, C, F>
where Self: Sync + Send,
      CTX: Send + Sync,
      REQ: DeserializeOwned + Send + Sync,
      RES: Serialize + Send + Sync,
      ERR: Serialize + Send + Sync,
      C: Fn(Arc<CTX>, REQ) -> F + Send + Sync,
      F: Future<Output=Result<RES,ERR>> + Send + Sync
{
    context: Arc<CTX>,
    callback: C,
    _marker1: PhantomData<REQ>,
    _marker2: PhantomData<RES>,
    _marker3: PhantomData<ERR>,
}

impl<CTX, REQ, RES, ERR, C, F> ServiceHandler<CTX, REQ, RES, ERR, C, F>
where Self: Sync + Send,
      CTX: Send + Sync,
      REQ: DeserializeOwned + Send + Sync,
      RES: Serialize + Send + Sync,
      ERR: Serialize + Send + Sync,
      C: Fn(Arc<CTX>, REQ) -> F + Send + Sync,
      F: Future<Output=Result<RES,ERR>> + Send + Sync
{
    pub fn new(context: Arc<CTX>, callback: C) -> Arc<ServiceHandler<CTX, REQ, RES, ERR, C, F>>
    {
        let ret = ServiceHandler {
            context,
            callback,
            _marker1: PhantomData,
            _marker2: PhantomData,
            _marker3: PhantomData
        };
        Arc::new(ret)
    }
}

#[async_trait]
impl<CTX, REQ, RES, ERR, C, F> ServiceInvoker
for ServiceHandler<CTX, REQ, RES, ERR, C, F>
where Self: Sync + Send,
      CTX: Send + Sync,
      REQ: DeserializeOwned + Send + Sync,
      RES: Serialize + Send + Sync,
      ERR: Serialize + Send + Sync,
      C: Fn(Arc<CTX>, REQ) -> F + Send + Sync,
      F: Future<Output=Result<RES,ERR>> + Send + Sync
{
    async fn invoke(&self, req: Bytes) -> Result<Result<Bytes, Bytes>, SerializationError>
    {
        let format = self.data_format();
        let req = format.deserialize::<REQ>(&req[..])?;

        let ctx = Arc::clone(&self.context);
        let ret = (self.callback)(ctx, req).await;

        let ret = match ret {
            Ok(res) => Ok(Bytes::from(format.serialize::<RES>(&res)?)),
            Err(err) => Err(Bytes::from(format.serialize::<ERR>(&err)?)),
        };
        Ok(ret)
    }

    fn data_format(&self) -> SerializationFormat
    {
        SerializationFormat::Json
    }

    fn request_type_name(&self) -> String
    {
        std::any::type_name::<REQ>().to_string()
    }

    fn response_type_name(&self) -> String
    {
        std::any::type_name::<RES>().to_string()
    }

    fn error_type_name(&self) -> String
    {
        std::any::type_name::<ServiceErrorReply<ERR>>().to_string()
    }
}