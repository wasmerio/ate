use ate_mio::mio::Port as InnerPort;
use std::io;
use std::ops::Deref;
use std::ops::DerefMut;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use ate::prelude::ChainKey;

use crate::api::InstanceClient;
use crate::model::SwitchHello;

#[derive(Debug, Clone)]
pub struct Port
{
    url: url::Url,
    chain: ChainKey,
    inner: InnerPort,
}

impl Port
{
    pub async fn new(url: url::Url, chain: ChainKey, access_token: String) -> io::Result<Port>
    {
        Self::new_ext(url, chain, access_token, false).await
    }

    pub async fn new_ext(url: url::Url, chain: ChainKey, access_token: String, no_inner_encryption: bool) -> io::Result<Port>
    {
        let client = InstanceClient::new_ext(url.clone(), "/net", false, no_inner_encryption)
            .await
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;
        let (mut tx, rx, ek) = client.split();

        let hello = SwitchHello {
            chain: chain.clone(),
            access_token,
            version: crate::model::PORT_COMMAND_VERSION,
        };

        let data = serde_json::to_vec(&hello)?;
        tx.send(&ek, &data[..]).await?;

        let tx = Box::new(tx);
        let rx = Box::new(rx);
        let port = InnerPort::new(tx, rx, ek).await?;

        Ok(
            Port {
                url,
                chain,
                inner: port
            }
        )
    }
}

impl Port
{
    pub fn chain(&self) -> &ChainKey {
        &self.chain
    }

    pub fn url(&self) -> &url::Url {
        &self.url
    }
}

impl Deref
for Port
{
    type Target = InnerPort;

    fn deref(&self) -> &InnerPort {
        &self.inner
    }
}

impl DerefMut
for Port
{
    fn deref_mut(&mut self) ->  &mut InnerPort {
        &mut self.inner
    }
}