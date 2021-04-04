use super::error::*;
use super::crypto::*;
use super::session::*;
use super::meta::*;
use snap::read::FrameDecoder;
use snap::read::FrameEncoder;
use bytes::{Bytes, Buf};
#[allow(unused_imports)]
use openssl::symm::{encrypt, decrypt, Cipher};

pub trait EventDataTransformer: Send + Sync
{
    /// Callback when data is stored in the event 
    fn data_as_underlay(&self, _meta: &mut Metadata, with: Bytes, _session: &Session) -> Result<Bytes, TransformError> {
        Ok(with)
    }

    /// Callback before data in an event is actually used by an actual user
    fn data_as_overlay(&self, _meta: &Metadata, with: Bytes, _session: &Session) -> Result<Bytes, TransformError> {
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
    fn data_as_underlay(&self, meta: &mut Metadata, with: Bytes, _session: &Session) -> Result<Bytes, TransformError> {
        let mut reader = FrameEncoder::new(with.reader());
        let mut compressed = Vec::new();
        std::io::copy(&mut reader, &mut compressed)?;
        Ok(Bytes::from(compressed))
    }

    #[allow(unused_variables)]
    fn data_as_overlay(&self, meta: &Metadata, with: Bytes, _session: &Session) -> Result<Bytes, TransformError> {
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
    fn data_as_underlay(&self, meta: &mut Metadata, with: Bytes, _session: &Session) -> Result<Bytes, TransformError>
    {
        let iv = meta.generate_iv();
        let encrypted = self.key.encrypt_with_iv(&iv, &with[..])?;
        Ok(Bytes::from(encrypted))
    }

    #[allow(unused_variables)]
    fn data_as_overlay(&self, meta: &Metadata, with: Bytes, _session: &Session) -> Result<Bytes, TransformError>
    {
        let iv = meta.get_iv()?;
        let decrypted = self.key.decrypt(&iv, &with[..])?;
        Ok(Bytes::from(decrypted))
    }
}

#[test]
fn test_encrypter()
{
    crate::utils::bootstrap_env();
    
    let key = EncryptKey::from_seed_string("test".to_string(), KeySize::Bit256);
    let encrypter = StaticEncryptionTransformer::new(&key);

    let test_bytes = Bytes::from_static(b"Some Crypto Text");
    let mut meta = Metadata::default();
    let encrypted = encrypter.data_as_underlay(&mut meta, test_bytes.clone(), &Session::default()).unwrap();

    println!("metadata: {:?}", meta);
    println!("data_test: {:X}", &test_bytes);
    println!("data_encrypted: {:X}", &encrypted);
    assert_ne!(&test_bytes, &encrypted);
    
    let decrypted = encrypter.data_as_overlay(&mut meta, encrypted, &Session::default()).unwrap();

    println!("data_decrypted: {:X}", &decrypted);
    assert_eq!(&test_bytes, &decrypted);
}

#[test]
fn test_compressor()
{
    let compressor = CompressorWithSnapTransformer::default();

    let test_bytes = Bytes::from("test".as_bytes());
    let mut meta = Metadata::default();
    let compressed = compressor.data_as_underlay(&mut meta, test_bytes.clone(), &Session::default()).unwrap();

    println!("metadata: {:?}", meta);
    println!("data_test: {:X}", &test_bytes);
    println!("data_compressed: {:X}", &compressed);
    assert_ne!(&test_bytes, &compressed);
    
    let decompressed = compressor.data_as_overlay(&mut meta, compressed, &Session::default()).unwrap();

    println!("data_decompressed: {:X}", &decompressed);
    assert_eq!(&test_bytes, &decompressed);
}

#[test]
fn test_crypto()
{
    let cipher = Cipher::aes_128_cbc();
    let data = b"Some Crypto Text";
    let key = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F";
    let iv = b"\x00\x01\x02\x03\x04\x05\x06\x07\x00\x01\x02\x03\x04\x05\x06\x07";
    let ciphertext = encrypt(
        cipher,
        key,
        Some(iv),
        data).unwrap();

    assert_eq!(
        b"\xB4\xB9\xE7\x30\xD6\xD6\xF7\xDE\x77\x3F\x1C\xFF\xB3\x3E\x44\x5A\x91\xD7\x27\x62\x87\x4D\
        \xFB\x3C\x5E\xC4\x59\x72\x4A\xF4\x7C\xA1",
        &ciphertext[..]);

    let cipher = Cipher::aes_128_cbc();
    let data = b"\xB4\xB9\xE7\x30\xD6\xD6\xF7\xDE\x77\x3F\x1C\xFF\xB3\x3E\x44\x5A\x91\xD7\x27\x62\
                \x87\x4D\xFB\x3C\x5E\xC4\x59\x72\x4A\xF4\x7C\xA1";
    let key = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F";
    let iv = b"\x00\x01\x02\x03\x04\x05\x06\x07\x00\x01\x02\x03\x04\x05\x06\x07";
    let ciphertext = decrypt(
        cipher,
        key,
        Some(iv),
        data).unwrap();

    assert_eq!(
        b"Some Crypto Text",
        &ciphertext[..]);
}