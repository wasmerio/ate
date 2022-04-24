use ate_crypto::EncryptKey;

#[async_trait::async_trait]
pub trait StreamReceiver
{
    async fn recv(&mut self, ek: &Option<EncryptKey>) -> Result<Vec<u8>, std::io::Error>;
}

#[async_trait::async_trait]
pub trait StreamTransmitter
{
    async fn send(&mut self, ek: &Option<EncryptKey>, data: &[u8]) -> Result<(), std::io::Error>;
}