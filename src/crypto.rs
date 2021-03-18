#[allow(unused_imports)]
use pqcrypto_falcon::ffi;
use serde::{Serialize, Deserialize};
use super::meta::*;
use super::error::*;
use rand::{RngCore, SeedableRng, rngs::adapter::ReseedingRng};
use rand_chacha::{ChaCha20Core, ChaCha20Rng};
use std::{cell::RefCell, io::ErrorKind};
use std::sync::{Mutex, MutexGuard};
use once_cell::sync::Lazy;
use std::result::Result;
#[allow(unused_imports)]
use pqcrypto_falcon::falcon512;
#[allow(unused_imports)]
use pqcrypto_falcon::falcon1024;
use pqcrypto_traits::sign::{DetachedSignature, PublicKey as PQCryptoPublicKey};
use pqcrypto_traits::sign::SecretKey as PQCryptoSecretKey;
#[allow(unused_imports)]
use openssl::symm::{Cipher};
#[allow(unused_imports)]
use openssl::error::{Error, ErrorStack};
#[allow(unused_imports)]
use sha3::Keccak256;
#[allow(unused_imports)]
use sha3::Digest;
use std::convert::TryInto;
use crate::conf::HashRoutine;

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

    pub fn encrypt_with_iv(&self, iv: &InitializationVector, data: &[u8]) -> Result<Vec<u8>, openssl::error::ErrorStack> {
        Ok(
            openssl::symm::encrypt(self.cipher(), self.value(), Some(&iv.bytes[..]), data)?
        )
    }

    pub fn encrypt(&self, data: &[u8]) -> Result<EncryptResult, std::io::Error> {
        let iv = InitializationVector::generate();
        let data = self.encrypt_with_iv(&iv, data)?;
        Ok(
            EncryptResult {
                iv: iv,
                data: data,
            }
        )
    }
    
    pub fn decrypt(&self, iv: &InitializationVector, data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
        Ok(
            openssl::symm::decrypt(self.cipher(), self.value(), Some(&iv.bytes[..]), data)?
        )
    }

    #[allow(dead_code)]
    pub fn as_bytes(&self) -> Vec<u8> {
        Vec::from(self.value())
    }

    #[allow(dead_code)]
    pub fn from_bytes(bytes: &[u8]) -> Result<EncryptKey, std::io::Error> {
        let bytes: Vec<u8> = Vec::from(bytes);
        match bytes.len() {
            16 => Ok(EncryptKey::Aes128(bytes.try_into().expect("Internal error while deserializing the Encryption Key"))),
            24 => Ok(EncryptKey::Aes192(bytes.try_into().expect("Internal error while deserializing the Encryption Key"))),
            32 => Ok(EncryptKey::Aes256(bytes.try_into().expect("Internal error while deserializing the Encryption Key"))),
            _ => Result::Err(std::io::Error::new(ErrorKind::Other, format!("The encryption key bytes are the incorrect length ({}).", bytes.len())))
        }
    }

    #[allow(dead_code)]
    pub fn hash(&self) -> Hash {
        match &self {
            EncryptKey::Aes128(a) => Hash::from_bytes(a),
            EncryptKey::Aes192(a) => Hash::from_bytes(a),
            EncryptKey::Aes256(a) => Hash::from_bytes(a),
        }
    }
}

pub struct EncryptResult {
    pub iv: InitializationVector,
    pub data: Vec<u8>
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct Hash {
    pub val: [u8; 16]
}

pub const HASH_ROUTINE:HashRoutine = if cfg!(use_blake3) {
    HashRoutine::Blake3
} else if cfg!(use_sha3) {
    HashRoutine::Sha3
} else {
    HashRoutine::Blake3
};

impl Hash {
    pub fn from_bytes(input: &[u8]) -> Hash {
        Self::from_bytes_by_routine(input, HASH_ROUTINE)
    }
    pub fn from_bytes_twice(input1: &[u8], input2: &[u8]) -> Hash {
        Self::from_bytes_twice_by_routine(input1, input2, HASH_ROUTINE)
    }
    fn from_bytes_by_routine(input: &[u8], routine: HashRoutine) -> Hash {
        match routine {
            HashRoutine::Blake3 => Hash::from_bytes_blake3(input),
            HashRoutine::Sha3 => Hash::from_bytes_sha3(input),
        }
    }
    fn from_bytes_twice_by_routine(input1: &[u8], input2: &[u8], routine: HashRoutine) -> Hash {
        match routine {
            HashRoutine::Blake3 => Hash::from_bytes_twice_blake3(input1, input2),
            HashRoutine::Sha3 => Hash::from_bytes_twice_sha3(input1, input2),
        }
    }
    fn from_bytes_blake3(input: &[u8]) -> Hash {
        let mut hasher = sha3::Keccak384::new();
        hasher.update(input);
        let result = hasher.finalize();
        let result: Vec<u8> = result.into_iter()
            .take(16)
            .collect();
        let result: [u8; 16] = result
            .try_into()
            .expect("The hash should hit into 16 bytes!");

        Hash {
            val: result,
        }
    }
    fn from_bytes_twice_blake3(input1: &[u8], input2: &[u8]) -> Hash {
        let mut hasher = sha3::Keccak384::new();
        hasher.update(input1);
        hasher.update(input2);
        let result = hasher.finalize();
        let result: Vec<u8> = result.into_iter()
            .take(16)
            .collect();
        let result: [u8; 16] = result
            .try_into()
            .expect("The hash should hit into 16 bytes!");

        Hash {
            val: result,
        }
    }
    fn from_bytes_sha3(input: &[u8]) -> Hash {
        let result: [u8; 32] = blake3::hash(input).into();
        let mut ret = Hash {
            val: Default::default(),
        };
        ret.val.copy_from_slice(&result[..16]);
        ret
    }
    fn from_bytes_twice_sha3(input1: &[u8], input2: &[u8]) -> Hash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(input1);
        hasher.update(input2);
        let result: [u8; 32] = hasher.finalize().into();
        let mut ret = Hash {
            val: Default::default(),
        };
        ret.val.copy_from_slice(&result[..16]);
        ret
    }

    pub fn to_string(&self) -> String {
        hex::encode(self.val)
    }
}

impl From<String>
for Hash
{
    fn from(val: String) -> Hash {
        Hash::from_bytes(val.as_bytes())
    }
}

impl From<&'static str>
for Hash
{
    fn from(val: &'static str) -> Hash {
        Hash::from(val.to_string())
    }
}

impl From<u64>
for Hash
{
    fn from(val: u64) -> Hash {
        Hash::from_bytes(&val.to_be_bytes())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub(crate) struct DoubleHash {
    hash1: Hash,
    hash2: Hash,
}

impl DoubleHash {
    #[allow(dead_code)]
    pub fn from_hashes(hash1: &Hash, hash2: &Hash) -> DoubleHash {
        DoubleHash {
            hash1: hash1.clone(),
            hash2: hash2.clone(),
        }
    }

    pub fn hash(&self) -> Hash {
        Hash::from_bytes_twice(&self.hash1.val[..], &self.hash2.val[..])
    }
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
pub(crate) struct RandomGeneratorAccessor { }

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

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub struct InitializationVector
{
    pub bytes: [u8; 16],
}

impl InitializationVector {
    pub fn generate() -> InitializationVector {
        let mut rng = RandomGeneratorAccessor::default();
        let mut iv = InitializationVector {
            bytes: [0 as u8; 16]
        };
        rng.fill_bytes(&mut iv.bytes);
        iv
    }
}

impl Metadata
{
    #[allow(dead_code)]
    pub fn generate_iv(&mut self) -> InitializationVector {
        let mut core = self.core.clone()
            .into_iter()
            .filter(|m|  match m {
                CoreMetadata::InitializationVector(_) => false,
                _ => true,
            })
            .collect::<Vec<_>>();
        
        let iv = InitializationVector::generate();
        core.push(CoreMetadata::InitializationVector(iv.clone()));
        self.core = core;
        return iv;
    }

    #[allow(dead_code)]
    pub fn get_iv(&self) -> Result<InitializationVector, CryptoError> {
        for m in self.core.iter() {
            match m {
                CoreMetadata::InitializationVector(iv) => return Result::Ok(iv.clone()),
                _ => { }
            }
        }
        Result::Err(CryptoError::NoIvPresent)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub enum PrivateKey {
    Falcon512 {
        pk: Vec<u8>,
        sk: Vec<u8>,
    },
    Falcon1024 {
        pk: Vec<u8>,
        sk: Vec<u8>,
    },
}

impl PrivateKey
{
    #[allow(dead_code)]
    pub fn generate(size: KeySize) -> PrivateKey {
        match size {
            KeySize::Bit128 | KeySize::Bit192 => {
                let (pk, sk) = falcon512::keypair();
                PrivateKey::Falcon512 {
                    pk: Vec::from(pk.as_bytes()),
                    sk: Vec::from(sk.as_bytes()),
                }
            },
            KeySize::Bit256 => {
                let (pk, sk) = falcon1024::keypair();
                PrivateKey::Falcon1024 {
                    pk: Vec::from(pk.as_bytes()),
                    sk: Vec::from(sk.as_bytes()),
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn as_public_key(&self) -> PublicKey {
        match &self {
            PrivateKey::Falcon512 { sk: _, pk } => {
                PublicKey::Falcon512 {
                    pk: pk.clone(),
                }
            },
            PrivateKey::Falcon1024 { sk: _, pk } => {
                PublicKey::Falcon1024 {
                    pk: pk.clone(),
                }
            },
        }
    }

    #[allow(dead_code)]
    pub fn hash(&self) -> Hash {
        match &self {
            PrivateKey::Falcon512 { pk, sk: _ } => Hash::from_bytes(&pk[..]),
            PrivateKey::Falcon1024 { pk, sk: _ } => Hash::from_bytes(&pk[..]),
        }
    }

    #[allow(dead_code)]
    pub fn pk(&self) -> Vec<u8> { 
        match &self {
            PrivateKey::Falcon512 { pk, sk: _ } => pk.clone(),
            PrivateKey::Falcon1024 { pk, sk: _ } => pk.clone(),
        }
    }

    #[allow(dead_code)]
    pub fn sk(&self) -> Vec<u8> { 
        match &self {
            PrivateKey::Falcon512 { pk: _, sk } => sk.clone(),
            PrivateKey::Falcon1024 { pk: _, sk } => sk.clone(),
        }
    }

    #[allow(dead_code)]
    pub fn sign(&self, data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
        let ret = match &self {
            PrivateKey::Falcon512 { pk: _, sk } => {
                let sk = match falcon512::SecretKey::from_bytes(&sk[..]) {
                    Ok(sk) => sk,
                    Err(err) => { return Result::Err(std::io::Error::new(ErrorKind::Other, format!("Failed to decode the secret key ({}).", err))); },
                };
                let sig = falcon512::detached_sign(data, &sk);
                Vec::from(sig.as_bytes())
            },
            PrivateKey::Falcon1024 { pk: _, sk } => {
                let sk = match falcon1024::SecretKey::from_bytes(&sk[..]) {
                    Ok(sk) => sk,
                    Err(err) => { return Result::Err(std::io::Error::new(ErrorKind::Other, format!("Failed to decode the secret key ({}).", err))); },
                };
                let sig = falcon1024::detached_sign(data, &sk);
                Vec::from(sig.as_bytes())
            },
        };
        
        Ok(ret)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub enum PublicKey {
    Falcon512 {
        pk: Vec<u8>,
    },
    Falcon1024 {
        pk: Vec<u8>,
    }
}

impl PublicKey
{
    #[allow(dead_code)]
    pub fn pk(&self) -> Vec<u8> { 
        match &self {
            PublicKey::Falcon512 { pk } => pk.clone(),
            PublicKey::Falcon1024 { pk } => pk.clone(),
        }
    }

    #[allow(dead_code)]
    pub fn hash(&self) -> Hash {
        match &self {
            PublicKey::Falcon512 { pk } => Hash::from_bytes(&pk[..]),
            PublicKey::Falcon1024 { pk } => Hash::from_bytes(&pk[..]),
        }
    }
    
    #[allow(dead_code)]
    pub fn verify(&self, data: &[u8], sig: &[u8]) -> Result<bool, pqcrypto_traits::Error> {
        let ret = match &self {
            PublicKey::Falcon512 { pk } => {
                let pk = falcon512::PublicKey::from_bytes(&pk[..])?;
                let sig = falcon512::DetachedSignature::from_bytes(sig)?;
                falcon512::verify_detached_signature(&sig, data, &pk).is_ok()
            },
            PublicKey::Falcon1024 { pk } => {
                let pk = falcon1024::PublicKey::from_bytes(&pk[..])?;
                let sig = falcon1024::DetachedSignature::from_bytes(sig)?;
                falcon1024::verify_detached_signature(&sig, data, &pk).is_ok()
            }
        };
        
        Ok(ret)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub struct EncryptedPrivateKey {
    pk: PublicKey,
    ek_hash: Hash,
    sk_iv: InitializationVector,
    sk_encrypted: Vec<u8>
}

impl EncryptedPrivateKey
{
    #[allow(dead_code)]
    pub fn generate(encrypt_key: &EncryptKey) -> Result<EncryptedPrivateKey, std::io::Error> {
        let pair = PrivateKey::generate(encrypt_key.size());
        EncryptedPrivateKey::from_pair(&pair, encrypt_key)
    }

    #[allow(dead_code)]
    pub fn from_pair(pair: &PrivateKey, encrypt_key: &EncryptKey) -> Result<EncryptedPrivateKey, std::io::Error> {
        let sk = pair.sk();
        let sk = encrypt_key.encrypt(&sk[..])?;
        
        Ok(
            EncryptedPrivateKey {
                pk: pair.as_public_key(),
                ek_hash: encrypt_key.hash(),
                sk_iv: sk.iv,
                sk_encrypted: sk.data,
            }
        )
    }

    #[allow(dead_code)]
    pub fn as_private_key(&self, key: &EncryptKey) -> Result<PrivateKey, std::io::Error> {
        let data = key.decrypt(&self.sk_iv, &self.sk_encrypted[..])?;
        match &self.pk {
            PublicKey::Falcon512 { pk } => {
                Ok(
                    PrivateKey::Falcon512 {
                        pk: pk.clone(),
                        sk: data,
                    }
                )
            },
            PublicKey::Falcon1024{ pk } => {
                Ok(
                    PrivateKey::Falcon1024 {
                        pk: pk.clone(),
                        sk: data,
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
    pub fn pk_hash(&self) -> Hash {
        self.pk.hash()
    }

    #[allow(dead_code)]
    pub(crate) fn double_hash(&self) -> DoubleHash {
        DoubleHash::from_hashes(&self.pk_hash(), &self.ek_hash)
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
fn test_asym_crypto_128()
{
    let key = EncryptKey::generate(KeySize::Bit128);
    let private = EncryptedPrivateKey::generate(&key).unwrap();
    let public = private.as_public_key();

    let plain = b"test";
    let sig = private.as_private_key(&key).unwrap().sign(plain).unwrap();
    assert!(public.verify(plain, &sig[..]).unwrap(), "Signature verificaton failed");

    let negative = b"blahtest";
    assert!(public.verify(negative, &sig[..]).unwrap() == false, "Signature verificaton passes when it should not");
}

#[test]
fn test_asym_crypto_256()
{
    let key = EncryptKey::generate(KeySize::Bit256);
    let private = EncryptedPrivateKey::generate(&key).unwrap();
    let public = private.as_public_key();

    let plain = b"test";
    let sig = private.as_private_key(&key).unwrap().sign(plain).unwrap();
    assert!(public.verify(plain, &sig[..]).unwrap(), "Signature verificaton failed");

    let negative = b"blahtest";
    assert!(public.verify(negative, &sig[..]).unwrap() == false, "Signature verificaton passes when it should not");
}