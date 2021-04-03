use async_trait::async_trait;
use crate::conf::ConfAte;
use super::OpenAction;
use super::OpenFlow;
use crate::chain::ChainKey;
use crate::conf::ChainOfTrustBuilder;
use crate::error::ChainCreationError;

pub struct OpenStaticBuilder
{
    builder: ChainOfTrustBuilder
}

impl OpenStaticBuilder
{
    fn new(builder: ChainOfTrustBuilder) -> OpenStaticBuilder {
        OpenStaticBuilder {
            builder: builder.clone(),
        }
    }

    pub fn all_persistent(cfg: &ConfAte) -> OpenStaticBuilder {
        let builder = ChainOfTrustBuilder::new(cfg).temporal(false);
        OpenStaticBuilder::new(builder)
    }

    pub fn all_ethereal(cfg: &ConfAte) -> OpenStaticBuilder {
        let builder = ChainOfTrustBuilder::new(cfg).temporal(true);
        OpenStaticBuilder::new(builder)
    }
}

#[async_trait]
impl OpenFlow
for OpenStaticBuilder
{
    async fn open(&self, _cfg: &ConfAte, _key: &ChainKey) -> Result<OpenAction, ChainCreationError> {
        Ok(OpenAction::Create(self.builder.clone()))
    }
}