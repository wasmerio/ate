use ate_mio::mio::Port as InnerPort;
use std::io;
use std::ops::Deref;
use std::ops::DerefMut;

use ate::prelude::ChainKey;

use crate::api::InstanceClient;
use crate::model::SwitchHello;

pub struct Port
{
    inner: InnerPort,
}

impl Port
{
    pub async fn new(url: url::Url, chain: ChainKey, access_token: String,) -> io::Result<Port>
    {
        let client = InstanceClient::new_ext(url, "/net", true).await
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
                inner: port
            }
        )
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