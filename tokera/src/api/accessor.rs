use std::sync::Arc;
use std::time::Duration;

use ate::prelude::*;
use crate::model::*;

pub struct TokApi
{
    pub dio: Arc<DioMut>,
    pub wallet: DaoMut<Wallet>,
    pub auth: url::Url,
    pub auth_cmd: Option<ChainGuard>,
    pub registry: Arc<Registry>,
    pub lock_timeout: Duration,
}

pub async fn build_api_accessor(dio: &Arc<DioMut>, wallet: DaoMut<Wallet>, auth: url::Url, registry: &Arc<Registry>) -> TokApi
{
    TokApi {
        dio: Arc::clone(dio),
        wallet,
        auth,
        auth_cmd: None,
        registry: Arc::clone(&registry),
        lock_timeout: Duration::from_millis(500),
    }
}

impl TokApi
{
    pub async fn commit(&mut self) -> Result<(), CommitError>
    {
        self.dio.commit().await?;
        Ok(())
    }

    pub fn session_identity(&self) -> String
    {
        self.dio.session().identity().to_string()
    }

    pub fn user_identity(&self) -> String
    {
        self.dio.session().user().identity().to_string()
    }
}