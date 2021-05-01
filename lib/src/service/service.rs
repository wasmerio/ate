#[allow(unused_imports)]
use log::{info, error, warn, debug};
use async_trait::async_trait;
use std::sync::Arc;

use crate::{error::*, event::*};
use crate::header::*;

use super::*;

pub type ServiceInstance<REQ, RES, ERR> = Arc<dyn ServiceHandler<REQ, RES, ERR> + Send + Sync>;

#[async_trait]
pub trait Service
where Self: Send + Sync
{
    fn filter(&self, evt: &EventData) -> bool;

    async fn notify(&self, key: PrimaryKey) -> Result<(), ServiceError<()>>;
}