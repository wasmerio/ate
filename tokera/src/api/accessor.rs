use std::sync::Arc;
use std::time::Duration;

use crate::model::*;
use ate::prelude::*;

#[derive(Clone)]
pub struct TokApi {
    pub dio: Arc<DioMut>,
    pub wallet: DaoMut<Wallet>,
    pub auth: url::Url,
    pub auth_cmd: Option<ChainGuard>,
    pub db_url: Option<url::Url>,
    pub registry: Arc<Registry>,
    pub lock_timeout: Duration,
}

pub async fn build_api_accessor(
    dio: &Arc<DioMut>,
    wallet: DaoMut<Wallet>,
    auth: url::Url,
    db_url: Option<url::Url>,
    registry: &Arc<Registry>,
) -> TokApi {
    TokApi {
        dio: Arc::clone(dio),
        wallet,
        auth,
        auth_cmd: None,
        db_url,
        registry: Arc::clone(&registry),
        lock_timeout: Duration::from_millis(500),
    }
}

impl TokApi {
    pub async fn commit(&mut self) -> Result<(), CommitError> {
        self.dio.commit().await?;
        Ok(())
    }

    pub fn remote<'a>(&'a self) -> Option<&'a url::Url> {
        self.dio.remote()
    }

    pub fn session<'a>(&'a self) -> DioSessionGuard<'a> {
        self.dio.session()
    }

    pub fn session_identity(&self) -> String {
        self.dio.session().identity().to_string()
    }

    pub fn user_identity(&self) -> String {
        self.dio.session().user().identity().to_string()
    }
}
