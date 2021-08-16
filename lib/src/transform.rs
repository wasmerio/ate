use super::error::*;
use super::crypto::*;
use super::session::*;
use super::meta::*;
use super::transaction::TransactionMetadata;
use snap::read::FrameDecoder;
use snap::read::FrameEncoder;
use bytes::{Bytes, Buf};

#[cfg(test)]
use super::conf::ConfAte;

pub trait EventDataTransformer: Send + Sync
{
    /// Callback when data is stored in the event 
    fn data_as_underlay(&self, _meta: &mut Metadata, with: Bytes, _session: &AteSession, _trans_meta: &TransactionMetadata) -> Result<Bytes, TransformError> {
        Ok(with)
    }

    /// Callback before data in an event is actually used by an actual user
    fn data_as_overlay(&self, _meta: &Metadata, with: Bytes, _session: &AteSession) -> Result<Bytes, TransformError> {
        Ok(with)
    }

    fn clone_transformer(&self) -> Box<dyn EventDataTransformer>;
}

#[derive(Debug, Default, Clone)]
pub struct CompressorWithSnapTransformer
{
}

impl EventDataTransformer
for CompressorWithSnapTransformer
{
    fn clone_transformer(&self) -> Box<dyn EventDataTransformer> {
        Box::new(self.clone())
    }

    #[allow(unused_variables)]
    fn data_as_underlay(&self, meta: &mut Metadata, with: Bytes, _session: &AteSession, _trans_meta: &TransactionMetadata) -> Result<Bytes, TransformError> {
        let mut reader = FrameEncoder::new(with.reader());
        let mut compressed = Vec::new();
        std::io::copy(&mut reader, &mut compressed)?;
        Ok(Bytes::from(compressed))
    }

    #[allow(unused_variables)]
    fn data_as_overlay(&self, meta: &Metadata, with: Bytes, _session: &AteSession) -> Result<Bytes, TransformError> {
        let mut reader = FrameDecoder::new(with.reader());
        let mut decompressed = Vec::new();
        std::io::copy(&mut reader, &mut decompressed)?;
        Ok(Bytes::from(decompressed))
    }
}

#[derive(Clone)]
pub struct StaticEncryptionTransformer
{
    key: EncryptKey,
}

impl StaticEncryptionTransformer
{
    #[allow(dead_code)]
    pub fn new(key: &EncryptKey) -> StaticEncryptionTransformer {
        StaticEncryptionTransformer {
            key: key.clone(),
        }
    }
}

impl EventDataTransformer
for StaticEncryptionTransformer
{
    fn clone_transformer(&self) -> Box<dyn EventDataTransformer> {
        Box::new(self.clone())
    }
    
    #[allow(unused_variables)]
    fn data_as_underlay(&self, meta: &mut Metadata, with: Bytes, _session: &AteSession, _trans_meta: &TransactionMetadata) -> Result<Bytes, TransformError>
    {
        let iv = meta.generate_iv();
        let encrypted = self.key.encrypt_with_iv(&iv, &with[..]);
        Ok(Bytes::from(encrypted))
    }

    #[allow(unused_variables)]
    fn data_as_overlay(&self, meta: &Metadata, with: Bytes, _session: &AteSession) -> Result<Bytes, TransformError>
    {
        let iv = meta.get_iv()?;
        let decrypted = self.key.decrypt(&iv, &with[..]);
        Ok(Bytes::from(decrypted))
    }
}

#[test]
fn test_encrypter()
{
    crate::utils::bootstrap_test_env();

    let key = EncryptKey::from_seed_string("test".to_string(), KeySize::Bit192);
    let encrypter = StaticEncryptionTransformer::new(&key);
    let cfg = ConfAte::default();

    let trans_meta = TransactionMetadata::default();
    let test_bytes = Bytes::from_static(b"Some Crypto Text");
    let mut meta = Metadata::default();
    let encrypted = encrypter.data_as_underlay(&mut meta, test_bytes.clone(), &AteSession::new(&cfg), &trans_meta).unwrap();

    println!("metadata: {:?}", meta);
    println!("data_test: {:X}", &test_bytes);
    println!("data_encrypted: {:X}", &encrypted);
    assert_ne!(&test_bytes, &encrypted);
    
    let decrypted = encrypter.data_as_overlay(&mut meta, encrypted, &AteSession::new(&cfg)).unwrap();

    println!("data_decrypted: {:X}", &decrypted);
    assert_eq!(&test_bytes, &decrypted);
}

#[test]
fn test_compressor()
{
    crate::utils::bootstrap_test_env();

    let compressor = CompressorWithSnapTransformer::default();
    let cfg = ConfAte::default();

    let trans_meta = TransactionMetadata::default();
    let test_bytes = Bytes::from("test".as_bytes());
    let mut meta = Metadata::default();
    let compressed = compressor.data_as_underlay(&mut meta, test_bytes.clone(), &AteSession::new(&cfg), &trans_meta).unwrap();

    println!("metadata: {:?}", meta);
    println!("data_test: {:X}", &test_bytes);
    println!("data_compressed: {:X}", &compressed);
    assert_ne!(&test_bytes, &compressed);
    
    let decompressed = compressor.data_as_overlay(&mut meta, compressed, &AteSession::new(&cfg)).unwrap();

    println!("data_decompressed: {:X}", &decompressed);
    assert_eq!(&test_bytes, &decompressed);
}

#[test]
fn test_crypto()
{
    crate::utils::bootstrap_test_env();
    
    let key = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F";
    let cipher = crate::crypto::EncryptKey::Aes128(key.clone());
    let data = b"Some Crypto Text";
    let iv = b"\x00\x01\x02\x03\x04\x05\x06\x07\x00\x01\x02\x03\x04\x05\x06\x07";
    let ciphertext = cipher.encrypt_with_iv(&InitializationVector::from(iv), data);

    assert_eq!(
        [110, 148, 177, 161, 48, 153, 25, 114, 206, 212, 126, 250, 70, 201, 154, 141],
        &ciphertext[..]);

    let key = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F";
    let cipher = crate::crypto::EncryptKey::Aes128(key.clone());
    let iv = b"\x00\x01\x02\x03\x04\x05\x06\x07\x00\x01\x02\x03\x04\x05\x06\x07";
    let data = cipher.decrypt(&InitializationVector::from(iv), &ciphertext[..]);
    
    assert_eq!(
        b"Some Crypto Text",
        &data[..]);
}