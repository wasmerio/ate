#[allow(unused_imports)]
use pqcrypto_falcon::ffi;
use serde::{Serialize, Deserialize};
use super::meta::*;
use super::error::*;
use rand::{RngCore, SeedableRng, rngs::adapter::ReseedingRng};
use rand_chacha::{ChaCha20Core, ChaCha20Rng};
use std::{cell::RefCell};
use std::sync::{Mutex, MutexGuard};
use once_cell::sync::Lazy;
use std::result::Result;
#[allow(unused_imports)]
use pqcrypto_falcon::falcon512;
#[allow(unused_imports)]
use pqcrypto_falcon::falcon1024;
use pqcrypto_traits::sign::PublicKey as PQCryptoPublicKey;
use pqcrypto_traits::sign::SecretKey as PQCryptoSecretKey;
#[allow(unused_imports)]
use openssl::symm::{Cipher};
#[allow(unused_imports)]
use openssl::error::{Error, ErrorStack};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum EncryptKey {
    Aes128([u8; 16]),
    Aes192([u8; 24]),
    Aes256([u8; 32]),
}

#[derive(Debug, Clone)]
pub enum KeySize {
    #[allow(dead_code)]
    Bit128,
    #[allow(dead_code)]
    Bit192,
    #[allow(dead_code)]
    Bit256,
}

impl EncryptKey {
    pub fn generate(size: KeySize) -> EncryptKey {
        THREAD_LOCAL_SECURE_AND_FAST_RANDOM.with(|s| {
            let rng = &mut s.borrow_mut().rng;
            match size {
                KeySize::Bit128 => {
                    let mut aes_key = [0; 16];
                    rng.fill_bytes(&mut aes_key);
                    EncryptKey::Aes128(aes_key)
                },
                KeySize::Bit192 => {
                    let mut aes_key = [0; 24];
                    rng.fill_bytes(&mut aes_key);
                    EncryptKey::Aes192(aes_key)
                },
                KeySize::Bit256 => {
                    let mut aes_key = [0; 32];
                    rng.fill_bytes(&mut aes_key);
                    EncryptKey::Aes256(aes_key)
                }
            }
        })
    }

    #[allow(dead_code)]
    pub fn from_string(str: String, size: KeySize) -> EncryptKey {
        let mut n = 0;
        let mut seed = [0 as u8; 32];
        for b in str.as_bytes() {
            seed[n] = *b;
            n = n + 1;
            if n >= 30 { break; }
        }

        let mut rng = ChaCha20Rng::from_seed(seed);
        match size {
            KeySize::Bit128 => {
                let mut aes_key = [0; 16];
                rng.fill_bytes(&mut aes_key);
                EncryptKey::Aes128(aes_key)
            },
            KeySize::Bit192 => {
                let mut aes_key = [0; 24];
                rng.fill_bytes(&mut aes_key);
                EncryptKey::Aes192(aes_key)
            },
            KeySize::Bit256 => {
                let mut aes_key = [0; 32];
                rng.fill_bytes(&mut aes_key);
                EncryptKey::Aes256(aes_key)
            }
        }
    }

    pub fn size(&self) -> KeySize {
        match self {
            EncryptKey::Aes128(_) => KeySize::Bit128,
            EncryptKey::Aes192(_) => KeySize::Bit192,
            EncryptKey::Aes256(_) => KeySize::Bit256,
        }
    }

    pub fn value(&self) -> &[u8] {
        match self {
            EncryptKey::Aes128(a) => a,
            EncryptKey::Aes192(a) => a,
            EncryptKey::Aes256(a) => a,
        }
    }

    pub fn cipher(&self) -> Cipher {
        match self.size() {
            KeySize::Bit128 => Cipher::aes_128_ctr(),
            KeySize::Bit192 => Cipher::aes_192_ctr(),
            KeySize::Bit256 => Cipher::aes_256_ctr(),
        }
    }

    pub fn encrypt_with_iv(&self, iv: &[u8], data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
        Ok(
            openssl::symm::encrypt(self.cipher(), self.value(), Some(iv), data)?
        )
    }

    pub fn encrypt(&self, data: &[u8]) -> Result<EncryptResult, std::io::Error> {
        let mut rng = RandomGeneratorAccessor::default();
        let mut iv = [0 as u8; 16];
        rng.fill_bytes(&mut iv);
        let iv = Vec::from(iv);

        let data = self.encrypt_with_iv(&iv, data)?;
        Ok(
            EncryptResult {
                iv: iv,
                data: data,
            }
        )
    }
    
    pub fn decrypt(&self, iv: &[u8], data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
        Ok(
            openssl::symm::decrypt(self.cipher(), self.value(), Some(iv), data)?
        )
    }
}

pub struct EncryptResult {
    pub iv: Vec<u8>,
    pub data: Vec<u8>
}

static GLOBAL_SECURE_AND_FAST_RANDOM: Lazy<Mutex<ChaCha20Rng>> = Lazy::new(|| {
    Mutex::new(ChaCha20Rng::from_entropy())
});

#[derive(Default)]
struct SingleThreadedSecureAndFastRandom {
}

impl<'a> SingleThreadedSecureAndFastRandom {
    fn lock(&'a mut self) -> MutexGuard<'static, ChaCha20Rng> {
        GLOBAL_SECURE_AND_FAST_RANDOM.lock().expect("Failed to create the crypto generator seedering engine")
    }
}

impl<'a> RngCore
for SingleThreadedSecureAndFastRandom {
    fn next_u32(&mut self) -> u32 {
        self.lock().next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        self.lock().next_u64()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.lock().fill_bytes(dest)
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        self.lock().try_fill_bytes(dest)
    }
}

struct SecureAndFastRandom
{
    rng: Box<dyn RngCore>,
}

impl SecureAndFastRandom {
    fn new() -> SecureAndFastRandom {
        let mut seeder = SingleThreadedSecureAndFastRandom::default();
        let rng = ChaCha20Core::from_rng(&mut seeder).expect("Failed to properly seed the secure random number generator");
        let reseeding_rng = ReseedingRng::new(rng, 0, seeder);
        SecureAndFastRandom {
            rng: Box::new(reseeding_rng),
        }
    }
}

thread_local! {
    static THREAD_LOCAL_SECURE_AND_FAST_RANDOM: RefCell<SecureAndFastRandom>
        = RefCell::new(SecureAndFastRandom::new());
}

#[derive(Default)]
pub struct RandomGeneratorAccessor { }

impl RngCore
for RandomGeneratorAccessor
{
    fn next_u32(&mut self) -> u32 {
        THREAD_LOCAL_SECURE_AND_FAST_RANDOM.with(|s| {
            s.borrow_mut().rng.next_u32()
        })
    }

    fn next_u64(&mut self) -> u64 {
        THREAD_LOCAL_SECURE_AND_FAST_RANDOM.with(|s| {
            s.borrow_mut().rng.next_u64()
        })
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        THREAD_LOCAL_SECURE_AND_FAST_RANDOM.with(|s| {
            s.borrow_mut().rng.fill_bytes(dest)
        })
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        THREAD_LOCAL_SECURE_AND_FAST_RANDOM.with(|s| {
            s.borrow_mut().rng.try_fill_bytes(dest)
        })
    }
}

impl Default for EncryptKey {
    fn default() -> EncryptKey {
        EncryptKey::generate(KeySize::Bit192)
    }
}

impl<M> Metadata<M>
where M: OtherMetadata
{
    #[allow(dead_code)]
    pub fn generate_iv(&mut self) -> Vec<u8> {
        let mut core = self.core.clone()
            .into_iter()
            .filter(|m|  match m {
                CoreMetadata::InitializationVector(_) => false,
                _ => true,
            })
            .collect::<Vec<_>>();
        
        let mut rng = RandomGeneratorAccessor::default();
        let mut iv = [0 as u8; 16];
        rng.fill_bytes(&mut iv);
        core.push(CoreMetadata::InitializationVector(iv.clone()));
        let ret = Vec::from(iv);

        self.core = core;
        return ret;
    }

    #[allow(dead_code)]
    pub fn get_iv(&self) -> Result<Vec<u8>, CryptoError> {
        for m in self.core.iter() {
            match m {
                CoreMetadata::InitializationVector(iv) => return Result::Ok(iv.to_vec()),
                _ => { }
            }
        }
        Result::Err(CryptoError::NoIvPresent)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub enum PrivateKey {
    Falcon512 {
        id: u64,
        pk: Vec<u8>,
        sk: Vec<u8>,
    },
    Falcon1024 {
        id: u64,
        pk: Vec<u8>,
        sk: Vec<u8>,
    },
}

impl PrivateKey
{
    #[allow(dead_code)]
    pub fn generate(size: KeySize) -> PrivateKey {
        let mut rng = RandomGeneratorAccessor::default();
        match size {
            KeySize::Bit128 | KeySize::Bit192 => {
                let (pk, sk) = falcon512::keypair();
                PrivateKey::Falcon512 {
                    id: rng.next_u64(),
                    pk: Vec::from(pk.as_bytes()),
                    sk: Vec::from(sk.as_bytes()),
                }
            },
            KeySize::Bit256 => {
                let (pk, sk) = falcon1024::keypair();
                PrivateKey::Falcon1024 {
                    id: rng.next_u64(),
                    pk: Vec::from(pk.as_bytes()),
                    sk: Vec::from(sk.as_bytes()),
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn as_public_key(&self) -> PublicKey {
        match &self {
            PrivateKey::Falcon512 { id, pk, sk: _ } => {
                PublicKey::Falcon512 {
                    id: id.clone(),
                    pk: pk.clone(),
                }
            },
            PrivateKey::Falcon1024 { id, pk, sk: _ } => {
                PublicKey::Falcon1024 {
                    id: id.clone(),
                    pk: pk.clone(),
                }
            },
        }
    }

    #[allow(dead_code)]
    pub fn id(&self) -> u64 {
        match &self {
            PrivateKey::Falcon512 { id, pk: _, sk: _ } => id.clone(),
            PrivateKey::Falcon1024 { id, pk: _, sk: _ } => id.clone(),
        }
    }

    #[allow(dead_code)]
    pub fn pk(&self) -> Vec<u8> { 
        match &self {
            PrivateKey::Falcon512 { id: _, pk, sk: _ } => pk.clone(),
            PrivateKey::Falcon1024 { id: _, pk, sk: _ } => pk.clone(),
        }
    }

    #[allow(dead_code)]
    pub fn sk(&self) -> Vec<u8> { 
        match &self {
            PrivateKey::Falcon512 { id: _, pk: _, sk } => sk.clone(),
            PrivateKey::Falcon1024 { id: _, pk: _, sk } => sk.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub enum PublicKey {
    Falcon512 {
        id: u64,
        pk: Vec<u8>
    },
    Falcon1024 {
        id: u64,
        pk: Vec<u8>
    }
}

impl PublicKey
{
    #[allow(dead_code)]
    pub fn pk(&self) -> Vec<u8> { 
        match &self {
            PublicKey::Falcon512 { id: _, pk } => pk.clone(),
            PublicKey::Falcon1024 { id: _, pk } => pk.clone(),
        }
    }

    #[allow(dead_code)]
    pub fn id(&self) -> u64 {
        match &self {
            PublicKey::Falcon512 { id, pk: _ } => id.clone(),
            PublicKey::Falcon1024 { id, pk: _ } => id.clone(),
        }
    }
}

pub struct EncryptedPrivateKey {
    pk: PublicKey,
    sk_iv: Vec<u8>,
    sk_encrypted: Vec<u8>
}

impl EncryptedPrivateKey
{
    #[allow(dead_code)]
    pub fn generate(encrypt_key: EncryptKey) -> Result<EncryptedPrivateKey, std::io::Error> {
        let k = PrivateKey::generate(encrypt_key.size());
        let sk = k.sk();
        let sk = encrypt_key.encrypt(&sk[..])?;

        Ok(
            EncryptedPrivateKey {
                pk: k.as_public_key(),
                sk_iv: sk.iv,
                sk_encrypted: sk.data,
            }
        )
    }

    #[allow(dead_code)]
    pub fn as_private_key(&self, key: &EncryptKey) -> Result<PrivateKey, std::io::Error> {
        let data = key.decrypt(&self.sk_iv[..], &self.sk_encrypted[..])?;
        match &self.pk {
            PublicKey::Falcon512 { id, pk } => {
                Ok(
                    PrivateKey::Falcon512 {
                        id: id.clone(),
                        pk: pk.clone(),
                        sk: data
                    }
                )
            },
            PublicKey::Falcon1024{ id, pk } => {
                Ok(
                    PrivateKey::Falcon1024 {
                        id: id.clone(),
                        pk: pk.clone(),
                        sk: data
                    }
                )
            },
        }
    }

    #[allow(dead_code)]
    pub fn as_public_key(&self) -> PublicKey {
        self.pk.clone()
    }

    #[allow(dead_code)]
    pub fn id(&self) -> u64 {
        self.pk.id()
    }
}

#[test]
fn test_secure_random() {
    let t = 1024;
    for _ in 0..t {
        let mut data = [0 as u8; 1024];
        RandomGeneratorAccessor::default().fill_bytes(&mut data);
    }
}

#[test]
fn test_encrypt_key_seeding() {
    let provided = EncryptKey::from_string("test".to_string(), KeySize::Bit256);
    let expected = EncryptKey::Aes256([109, 23, 234, 219, 133, 97, 152, 126, 236, 229, 197, 134, 107, 89, 217, 82, 107, 27, 70, 176, 239, 71, 218, 171, 68, 75, 54, 215, 249, 219, 231, 69]);
    assert_eq!(provided, expected);

    let provided = EncryptKey::from_string("test2".to_string(), KeySize::Bit256);
    let expected = EncryptKey::Aes256([230, 248, 163, 17, 228, 69, 199, 43, 44, 106, 137, 243, 229, 187, 80, 173, 250, 183, 169, 165, 247, 153, 250, 8, 248, 187, 48, 83, 165, 91, 255, 180]);
    assert_eq!(provided, expected);
}

#[test]
fn test_asym_crypto() {
    let plain = b"test";
    let (pk, sk) = falcon512::keypair();
    let sig = falcon512::detached_sign(plain, &sk);
    assert!(falcon512::verify_detached_signature(&sig, plain, &pk).is_ok());
}