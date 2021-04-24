#![allow(unused_imports)]
use log::{info, error, debug};
use fxhash::FxHashMap;
use pqcrypto_falcon::ffi;
use serde::{Serialize, Deserialize};
use super::meta::*;
use super::error::*;
use rand::{RngCore, SeedableRng, rngs::adapter::ReseedingRng};
use rand_chacha::{ChaCha20Core, ChaCha20Rng};
use std::{cell::RefCell, io::ErrorKind, marker::PhantomData};
use std::sync::{Mutex, MutexGuard};
use once_cell::sync::Lazy;
use std::result::Result;
use pqcrypto_ntru::ntruhps2048509 as ntru128;
use pqcrypto_ntru::ntruhps2048677 as ntru192;
use pqcrypto_ntru::ntruhps4096821 as ntru256;
use pqcrypto_ntru::ffi::*;
use pqcrypto_falcon::falcon512;
use pqcrypto_falcon::falcon1024;
use pqcrypto_traits::sign::{DetachedSignature, PublicKey as PQCryptoPublicKey};
use pqcrypto_traits::sign::SecretKey as PQCryptoSecretKey;
use pqcrypto_traits::kem::*;
use openssl::symm::{Cipher};
use openssl::error::{Error, ErrorStack};
use sha3::Keccak256;
use sha3::Digest;
use std::convert::TryInto;
use crate::conf::HashRoutine;
use crate::spec::SerializationFormat;

/// Size of a cryptographic key, smaller keys are still very secure but
/// have less room in the future should new attacks be found against the
/// crypto algorithms used by ATE.
#[repr(u8)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KeySize {
    #[allow(dead_code)]
    Bit128 = 16,
    #[allow(dead_code)]
    Bit192 = 24,
    #[allow(dead_code)]
    Bit256 = 32,
}

impl KeySize
{
    pub fn ntru_public_key_size(&self) -> usize {
        match &self {
            KeySize::Bit128 => ntru128::public_key_bytes(),
            KeySize::Bit192 => ntru192::public_key_bytes(),
            KeySize::Bit256 => ntru256::public_key_bytes(),
        }
    }

    pub fn ntru_private_key_size(&self) -> usize {
        match &self {
            KeySize::Bit128 => ntru128::secret_key_bytes(),
            KeySize::Bit192 => ntru192::secret_key_bytes(),
            KeySize::Bit256 => ntru256::secret_key_bytes(),
        }
    }

    pub fn ntru_cipher_text_size(&self) -> usize {
        match &self {
            KeySize::Bit128 => ntru128::ciphertext_bytes(),
            KeySize::Bit192 => ntru192::ciphertext_bytes(),
            KeySize::Bit256 => ntru256::ciphertext_bytes(),
        }
    }
}

impl std::str::FromStr
for KeySize
{
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "128" => Ok(KeySize::Bit128),
            "192" => Ok(KeySize::Bit192),
            "256" => Ok(KeySize::Bit256),
            _ => Err("valid values are '128', '192', '256'"),
        }
    }
}

impl std::fmt::Display
for KeySize
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeySize::Bit128 => write!(f, "128"),
            KeySize::Bit192 => write!(f, "192"),
            KeySize::Bit256 => write!(f, "256"),
        }
    }
}

/// Represents an encryption key that will give confidentiality to
/// data stored within the redo-log. Note this does not give integrity
/// which comes from the `PrivateKey` crypto instead.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum EncryptKey {
    Aes128([u8; 16]),
    Aes192([u8; 24]),
    Aes256([u8; 32]),
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

    #[deprecated(
        since = "0.2.1",
        note = "Please use 'from_seed_string' instead as this function only uses the first 30 bytes while the later uses all of the string as its entropy."
    )]
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
        let iv_store;
        let iv = match iv.bytes.len() {
            16 => iv,
            _ => {
                iv_store = InitializationVector {
                    bytes: iv.bytes.clone().into_iter().take(16).collect::<Vec<_>>()
                };
                &iv_store
            }
        };
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
        let iv_store;
        let iv = match iv.bytes.len() {
            16 => iv,
            _ => {
                iv_store = InitializationVector {
                    bytes: iv.bytes.clone().into_iter().take(16).collect::<Vec<_>>()
                };
                &iv_store
            }
        };
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

    #[allow(dead_code)]
    pub fn from_seed_string(str: String, size: KeySize) -> EncryptKey {
        EncryptKey::from_seed_bytes(str.as_bytes(), size)
    }

    #[allow(dead_code)]
    pub fn from_seed_bytes(seed_bytes: &[u8], size: KeySize) -> EncryptKey
    {
        let mut hasher = sha3::Keccak384::new();
        hasher.update(seed_bytes);
        let result = hasher.finalize();

        match size {
            KeySize::Bit128 => {
                let aes_key: [u8; 16] = result.into_iter().take(16).collect::<Vec<_>>().try_into().unwrap();
                EncryptKey::Aes128(aes_key)
            },
            KeySize::Bit192 => {
                let aes_key: [u8; 24] = result.into_iter().take(24).collect::<Vec<_>>().try_into().unwrap();
                EncryptKey::Aes192(aes_key)
            },
            KeySize::Bit256 => {
                let aes_key: [u8; 32] = result.into_iter().take(32).collect::<Vec<_>>().try_into().unwrap();
                EncryptKey::Aes256(aes_key)
            }
        }
    }

    pub fn xor(ek1: EncryptKey, ek2: EncryptKey) -> Result<EncryptKey, std::io::Error>
    {
        let mut ek1_bytes = ek1.as_bytes();
        let ek2_bytes = ek2.as_bytes();

        ek1_bytes.iter_mut()
            .zip(ek2_bytes.iter())
            .for_each(|(x1, x2)| *x1 ^= *x2);

        EncryptKey::from_bytes(&ek1_bytes[..])
    }
}

impl std::fmt::Display
for EncryptKey
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncryptKey::Aes128(a) => write!(f, "aes-128:{}", hex::encode(a)),
            EncryptKey::Aes192(a) => write!(f, "aes-192:{}", hex::encode(a)),
            EncryptKey::Aes256(a) => write!(f, "aes-256:{}", hex::encode(a)),
        }
    }
}

pub struct EncryptResult {
    pub iv: InitializationVector,
    pub data: Vec<u8>
}

/// Represents a hash of a piece of data that is cryptographically secure enough
/// that it can be used for integrity but small enough that it does not bloat
/// the redo log metadata.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct Hash {
    pub val: [u8; 16]
}

impl Hash {
    pub fn from_bytes(input: &[u8]) -> Hash {
        Self::from_bytes_by_routine(input, crate::HASH_ROUTINE)
    }
    pub fn from_bytes_twice(input1: &[u8], input2: &[u8]) -> Hash {
        Self::from_bytes_twice_by_routine(input1, input2, crate::HASH_ROUTINE)
    }
    fn from_bytes_by_routine(input: &[u8], routine: HashRoutine) -> Hash {
        match routine {
            HashRoutine::Blake3 => Hash::from_bytes_blake3(input),
            HashRoutine::Sha3 => Hash::from_bytes_sha3(input, 1),
        }
    }
    fn from_bytes_twice_by_routine(input1: &[u8], input2: &[u8], routine: HashRoutine) -> Hash {
        match routine {
            HashRoutine::Blake3 => Hash::from_bytes_twice_blake3(input1, input2),
            HashRoutine::Sha3 => Hash::from_bytes_twice_sha3(input1, input2),
        }
    }
    pub fn from_bytes_sha3(input: &[u8], repeat: i32) -> Hash {
        let mut hasher = sha3::Keccak384::new();
        for _ in 0..repeat {
            hasher.update(input);
        }
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
    fn from_bytes_twice_sha3(input1: &[u8], input2: &[u8]) -> Hash {
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
    pub fn from_bytes_blake3(input: &[u8]) -> Hash {
        let result: [u8; 32] = blake3::hash(input).into();
        let mut ret = Hash {
            val: Default::default(),
        };
        ret.val.copy_from_slice(&result[..16]);
        ret
    }
    fn from_bytes_twice_blake3(input1: &[u8], input2: &[u8]) -> Hash {
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

    pub fn to_hex_string(&self) -> String {
        hex::encode(self.val)
    }

    pub fn to_string(&self) -> String {
        self.to_hex_string()
    }

    pub fn to_bytes(&self) -> &[u8; 16] {
        &self.val
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

impl std::fmt::Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
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

/// Represents an initiailization vector used for both hash prefixing
/// to create entropy and help prevent rainbow table attacks. These
/// vectors are also used as the exchange medium during a key exchange
/// so that two parties can established a shared secret key
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub struct InitializationVector
{
    pub bytes: Vec<u8>,
}

impl InitializationVector {
    pub fn generate() -> InitializationVector {
        let mut rng = RandomGeneratorAccessor::default();
        let mut iv = InitializationVector {
            bytes: vec![0 as u8; 16]
        };
        rng.fill_bytes(&mut iv.bytes);
        iv
    }

    pub fn from_bytes(bytes: Vec<u8>) -> InitializationVector {
        InitializationVector {
            bytes,
        }
    }
}

impl std::fmt::Display
for InitializationVector
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(&self.bytes[..]))
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
    pub fn get_iv(&self) -> Result<&InitializationVector, CryptoError> {
        for m in self.core.iter() {
            match m {
                CoreMetadata::InitializationVector(iv) => return Result::Ok(iv),
                _ => { }
            }
        }
        Result::Err(CryptoError::NoIvPresent)
    }
}

/// Private keys provide the ability to sign records within the
/// redo log chain-of-trust, these inserts records with associated
/// public keys embedded within teh cahin allow
/// records/events stored within the ATE redo log to have integrity
/// without actually being able to read the records themselves. This
/// attribute allows a chain-of-trust to be built without access to
/// the data held within of chain. Asymetric crypto in ATE uses the
/// leading candidates from NIST that provide protection against
/// quantom computer attacks
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub enum PrivateSignKey {
    Falcon512 {
        pk: Vec<u8>,
        sk: Vec<u8>,
    },
    Falcon1024 {
        pk: Vec<u8>,
        sk: Vec<u8>,
    },
}

impl PrivateSignKey
{
    #[allow(dead_code)]
    pub fn generate(size: KeySize) -> PrivateSignKey {
        match size {
            KeySize::Bit128 | KeySize::Bit192 => {
                let (pk, sk) = falcon512::keypair();
                PrivateSignKey::Falcon512 {
                    pk: Vec::from(pk.as_bytes()),
                    sk: Vec::from(sk.as_bytes()),
                }
            },
            KeySize::Bit256 => {
                let (pk, sk) = falcon1024::keypair();
                PrivateSignKey::Falcon1024 {
                    pk: Vec::from(pk.as_bytes()),
                    sk: Vec::from(sk.as_bytes()),
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn as_public_key(&self) -> PublicSignKey {
        match &self {
            PrivateSignKey::Falcon512 { sk: _, pk } => {
                PublicSignKey::Falcon512 {
                    pk: pk.clone(),
                }
            },
            PrivateSignKey::Falcon1024 { sk: _, pk } => {
                PublicSignKey::Falcon1024 {
                    pk: pk.clone(),
                }
            },
        }
    }

    #[allow(dead_code)]
    pub fn hash(&self) -> Hash {
        match &self {
            PrivateSignKey::Falcon512 { pk, sk: _ } => Hash::from_bytes(&pk[..]),
            PrivateSignKey::Falcon1024 { pk, sk: _ } => Hash::from_bytes(&pk[..]),
        }
    }

    #[allow(dead_code)]
    pub fn pk(&self) -> Vec<u8> { 
        match &self {
            PrivateSignKey::Falcon512 { pk, sk: _ } => pk.clone(),
            PrivateSignKey::Falcon1024 { pk, sk: _ } => pk.clone(),
        }
    }

    #[allow(dead_code)]
    pub fn sk(&self) -> Vec<u8> { 
        match &self {
            PrivateSignKey::Falcon512 { pk: _, sk } => sk.clone(),
            PrivateSignKey::Falcon1024 { pk: _, sk } => sk.clone(),
        }
    }

    #[allow(dead_code)]
    pub fn sign(&self, data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
        let ret = match &self {
            PrivateSignKey::Falcon512 { pk: _, sk } => {
                let sk = match falcon512::SecretKey::from_bytes(&sk[..]) {
                    Ok(sk) => sk,
                    Err(err) => { return Result::Err(std::io::Error::new(ErrorKind::Other, format!("Failed to decode the secret key ({}).", err))); },
                };
                let sig = falcon512::detached_sign(data, &sk);
                Vec::from(sig.as_bytes())
            },
            PrivateSignKey::Falcon1024 { pk: _, sk } => {
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

impl std::fmt::Display
for PrivateSignKey
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrivateSignKey::Falcon512 { pk: _, sk: _ } => write!(f, "falcon512:pk:{}+sk", self.hash()),
            PrivateSignKey::Falcon1024 { pk: _, sk: _ } => write!(f, "falcon1024:pk:{}+sk", self.hash()),
        }
    }
}

/// Public key which is one side of a private key. Public keys allow
/// records/events stored within the ATE redo log to have integrity
/// without actually being able to read the records themselves. This
/// attribute allows a chain-of-trust to be built without access to
/// the data held within of chain. Asymetric crypto in ATE uses the
/// leading candidates from NIST that provide protection against
/// quantom computer attacks
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub enum PublicSignKey {
    Falcon512 {
        pk: Vec<u8>,
    },
    Falcon1024 {
        pk: Vec<u8>,
    }
}

impl PublicSignKey
{
    #[allow(dead_code)]
    pub fn pk(&self) -> Vec<u8> { 
        match &self {
            PublicSignKey::Falcon512 { pk } => pk.clone(),
            PublicSignKey::Falcon1024 { pk } => pk.clone(),
        }
    }

    #[allow(dead_code)]
    pub fn hash(&self) -> Hash {
        match &self {
            PublicSignKey::Falcon512 { pk } => Hash::from_bytes(&pk[..]),
            PublicSignKey::Falcon1024 { pk } => Hash::from_bytes(&pk[..]),
        }
    }
    
    #[allow(dead_code)]
    pub fn verify(&self, data: &[u8], sig: &[u8]) -> Result<bool, pqcrypto_traits::Error> {
        let ret = match &self {
            PublicSignKey::Falcon512 { pk } => {
                let pk = falcon512::PublicKey::from_bytes(&pk[..])?;
                let sig = falcon512::DetachedSignature::from_bytes(sig)?;
                falcon512::verify_detached_signature(&sig, data, &pk).is_ok()
            },
            PublicSignKey::Falcon1024 { pk } => {
                let pk = falcon1024::PublicKey::from_bytes(&pk[..])?;
                let sig = falcon1024::DetachedSignature::from_bytes(sig)?;
                falcon1024::verify_detached_signature(&sig, data, &pk).is_ok()
            }
        };
        
        Ok(ret)
    }
}

impl std::fmt::Display
for PublicSignKey
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PublicSignKey::Falcon512 { pk: _ } => write!(f, "falcon512:pk:{}", self.hash()),
            PublicSignKey::Falcon1024 { pk: _ } => write!(f, "falcon1024:pk:{}", self.hash()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub struct EncryptedPrivateKey {
    pk: PublicSignKey,
    ek_hash: Hash,
    sk_iv: InitializationVector,
    sk_encrypted: Vec<u8>
}

impl EncryptedPrivateKey
{
    #[allow(dead_code)]
    pub fn generate(encrypt_key: &EncryptKey) -> Result<EncryptedPrivateKey, std::io::Error> {
        let pair = PrivateSignKey::generate(encrypt_key.size());
        EncryptedPrivateKey::from_pair(&pair, encrypt_key)
    }

    #[allow(dead_code)]
    pub fn from_pair(pair: &PrivateSignKey, encrypt_key: &EncryptKey) -> Result<EncryptedPrivateKey, std::io::Error> {
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
    pub fn as_private_key(&self, key: &EncryptKey) -> Result<PrivateSignKey, std::io::Error> {
        let data = key.decrypt(&self.sk_iv, &self.sk_encrypted[..])?;
        match &self.pk {
            PublicSignKey::Falcon512 { pk } => {
                Ok(
                    PrivateSignKey::Falcon512 {
                        pk: pk.clone(),
                        sk: data,
                    }
                )
            },
            PublicSignKey::Falcon1024{ pk } => {
                Ok(
                    PrivateSignKey::Falcon1024 {
                        pk: pk.clone(),
                        sk: data,
                    }
                )
            },
        }
    }

    #[allow(dead_code)]
    pub fn as_public_key(&self) -> PublicSignKey {
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EncryptedSecureData<T>
where T: serde::Serialize + serde::de::DeserializeOwned
{
    format: SerializationFormat,
    ek_hash: Hash,
    sd_iv: InitializationVector,
    sd_encrypted: Vec<u8>,
    #[serde(skip)]
    _marker: std::marker::PhantomData<T>,
}

impl<T> EncryptedSecureData<T>
where T: serde::Serialize + serde::de::DeserializeOwned
{
    pub fn new(encrypt_key: &PublicEncryptKey, data: T) -> Result<EncryptedSecureData<T>, std::io::Error> {
        let format = SerializationFormat::Bincode;
        let data = match format.serialize(&data) {
            Ok(a) => a,
            Err(err) => { return Err(std::io::Error::new(ErrorKind::Other, err.to_string())); }
        };
        let result = encrypt_key.encrypt(&data[..])?;
        
        Ok(
            EncryptedSecureData {
                format,
                ek_hash: encrypt_key.hash(),
                sd_iv: result.iv,
                sd_encrypted: result.data,
                _marker: PhantomData,
            }
        )
    }

    pub fn unwrap(&self, key: &PrivateEncryptKey) -> Result<T, std::io::Error> {
        let data = key.decrypt(&self.sd_iv, &self.sd_encrypted[..])?;
        Ok(match self.format.deserialize(&data[..]) {
            Ok(a) => a,
            Err(err) => { return Err(std::io::Error::new(ErrorKind::Other, err.to_string())); }
        })
    }

    pub fn ek_hash(&self) -> Hash {
        self.ek_hash
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MultiEncryptedSecureData<T>
where T: serde::Serialize + serde::de::DeserializeOwned
{
    format: SerializationFormat,
    members: FxHashMap<String, EncryptedSecureData<EncryptKey>>,
    metadata: FxHashMap<String, String>,
    sd_iv: InitializationVector,
    sd_encrypted: Vec<u8>,
    #[serde(skip)]
    _marker2: std::marker::PhantomData<T>,
}

impl<T> MultiEncryptedSecureData<T>
where T: serde::Serialize + serde::de::DeserializeOwned
{
    pub fn new(encrypt_key: &PublicEncryptKey, meta: String, data: T) -> Result<MultiEncryptedSecureData<T>, std::io::Error> {
        let format = SerializationFormat::Bincode;
        let shared_key = EncryptKey::generate(encrypt_key.size());
        
        let index = encrypt_key.hash().to_hex_string();
        let mut members = FxHashMap::default();
        members.insert(index.clone(), EncryptedSecureData::new(encrypt_key, shared_key)?);
        let mut metadata = FxHashMap::default();
        metadata.insert(index, meta);

        let data = match format.serialize(&data) {
            Ok(a) => a,
            Err(err) => { return Err(std::io::Error::new(ErrorKind::Other, err.to_string())); }
        };
        let result = shared_key.encrypt(&data[..])?;
        
        Ok(
            MultiEncryptedSecureData {
                format,
                members,
                metadata,
                sd_iv: result.iv,
                sd_encrypted: result.data,
                _marker2: PhantomData,
            }
        )
    }

    pub fn unwrap(&self, key: &PrivateEncryptKey) -> Result<Option<T>, std::io::Error> {
        Ok(
            match self.members.get(&key.hash().to_hex_string()) {
                Some(a) => {
                    let shared_key = a.unwrap(key)?;
                    let data = shared_key.decrypt(&self.sd_iv, &self.sd_encrypted[..])?;
                    Some(match self.format.deserialize::<T>(&data[..]) {
                        Ok(a) => a,
                        Err(err) => { return Err(std::io::Error::new(ErrorKind::Other, err.to_string())); }
                    })
                },
                None => None
            }
        )
    }

    pub fn add(&mut self, encrypt_key: &PublicEncryptKey, meta: String, referrer: &PrivateEncryptKey) -> Result<bool, std::io::Error> {
        match self.members.get(&referrer.hash().to_hex_string()) {
            Some(a) => {
                let shared_key = a.unwrap(referrer)?;
                let index = encrypt_key.hash().to_hex_string();
                self.members.insert(index.clone(), EncryptedSecureData::new(encrypt_key, shared_key)?);
                self.metadata.insert(index, meta);
                Ok(true)
            },
            None => Ok(false)
        }
    }

    pub fn remove(&mut self, what: &Hash) -> bool {
        let index = what.to_hex_string();
        let ret = self.members.remove(&index).is_some();
        self.metadata.remove(&index);
        ret
    }

    pub fn exists(&self, what: &Hash) -> bool {
        let what = what.to_hex_string();
        self.members.contains_key(&what)
    }

    pub fn meta<'a>(&'a self, what: &Hash) -> Option<&'a String> {
        let index = what.to_hex_string();
        self.metadata.get(&index)
    }

    pub fn meta_list<'a>(&'a self) -> impl Iterator<Item = &'a String> {
        self.metadata.values()
    }
}

/// Private encryption keys provide the ability to decrypt a secret
/// that was encrypted using a Public Key - this capability is
/// useful for key-exchange and trust validation in the crypto chain.
/// Asymetric crypto in ATE uses the leading candidates from NIST
/// that provide protection against quantom computer attacks
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub enum PrivateEncryptKey {
    Ntru128 {
        pk: Vec<u8>,
        sk: Vec<u8>,
    },
    Ntru192 {
        pk: Vec<u8>,
        sk: Vec<u8>,
    },
    Ntru256 {
        pk: Vec<u8>,
        sk: Vec<u8>,
    },
}

impl PrivateEncryptKey
{
    #[allow(dead_code)]
    pub fn generate(size: KeySize) -> PrivateEncryptKey {
        match size {
            KeySize::Bit128 => {
                let (pk, sk) = ntru128::keypair();
                PrivateEncryptKey::Ntru128 {
                    pk: Vec::from(pk.as_bytes()),
                    sk: Vec::from(sk.as_bytes()),
                }
            },
            KeySize::Bit192 => {
                let (pk, sk) = ntru192::keypair();
                PrivateEncryptKey::Ntru192 {
                    pk: Vec::from(pk.as_bytes()),
                    sk: Vec::from(sk.as_bytes()),
                }
            },
            KeySize::Bit256 => {
                let (pk, sk) = ntru256::keypair();
                PrivateEncryptKey::Ntru256 {
                    pk: Vec::from(pk.as_bytes()),
                    sk: Vec::from(sk.as_bytes()),
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn as_public_key(&self) -> PublicEncryptKey {
        match &self {
            PrivateEncryptKey::Ntru128 { sk: _, pk } => {
                PublicEncryptKey::Ntru128 {
                    pk: pk.clone(),
                }
            },
            PrivateEncryptKey::Ntru192 { sk: _, pk } => {
                PublicEncryptKey::Ntru192 {
                    pk: pk.clone(),
                }
            },
            PrivateEncryptKey::Ntru256 { sk: _, pk } => {
                PublicEncryptKey::Ntru256 {
                    pk: pk.clone(),
                }
            },
        }
    }

    #[allow(dead_code)]
    pub fn hash(&self) -> Hash {
        match &self {
            PrivateEncryptKey::Ntru128 { pk, sk: _ } => Hash::from_bytes(&pk[..]),
            PrivateEncryptKey::Ntru192 { pk, sk: _ } => Hash::from_bytes(&pk[..]),
            PrivateEncryptKey::Ntru256 { pk, sk: _ } => Hash::from_bytes(&pk[..]),
        }
    }

    #[allow(dead_code)]
    pub fn pk(&self) -> Vec<u8> { 
        match &self {
            PrivateEncryptKey::Ntru128 { pk, sk: _ } => pk.clone(),
            PrivateEncryptKey::Ntru192 { pk, sk: _ } => pk.clone(),
            PrivateEncryptKey::Ntru256 { pk, sk: _ } => pk.clone(),
        }
    }

    #[allow(dead_code)]
    pub fn sk(&self) -> Vec<u8> { 
        match &self {
            PrivateEncryptKey::Ntru128 { pk: _, sk } => sk.clone(),
            PrivateEncryptKey::Ntru192 { pk: _, sk } => sk.clone(),
            PrivateEncryptKey::Ntru256 { pk: _, sk } => sk.clone(),
        }
    }

    #[allow(dead_code)]
    pub fn decapsulate(&self, iv: &InitializationVector) -> Option<EncryptKey> {
        match &self {
            PrivateEncryptKey::Ntru128 { pk: _, sk } => {
                if iv.bytes.len() != ntru128::ciphertext_bytes() { return None; }
                let ct = ntru128::Ciphertext::from_bytes(&iv.bytes[..]).unwrap();
                let sk = ntru128::SecretKey::from_bytes(&sk[..]).unwrap();
                let ss = ntru128::decapsulate(&ct, &sk);
                Some(EncryptKey::from_seed_bytes(ss.as_bytes(), KeySize::Bit128))
            },
            PrivateEncryptKey::Ntru192 { pk: _, sk } => {
                if iv.bytes.len() != ntru192::ciphertext_bytes() { return None; }
                let ct = ntru192::Ciphertext::from_bytes(&iv.bytes[..]).unwrap();
                let sk = ntru192::SecretKey::from_bytes(&sk[..]).unwrap();
                let ss = ntru192::decapsulate(&ct, &sk);
                Some(EncryptKey::from_seed_bytes(ss.as_bytes(), KeySize::Bit192))
            },
            PrivateEncryptKey::Ntru256 { pk: _, sk } => {
                if iv.bytes.len() != ntru256::ciphertext_bytes() { return None; }
                let ct = ntru256::Ciphertext::from_bytes(&iv.bytes[..]).unwrap();
                let sk = ntru256::SecretKey::from_bytes(&sk[..]).unwrap();
                let ss = ntru256::decapsulate(&ct, &sk);
                Some(EncryptKey::from_seed_bytes(ss.as_bytes(), KeySize::Bit256))
            },
        }
    }
    
    pub fn decrypt(&self, iv: &InitializationVector, data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
        let ek = match self.decapsulate(iv) {
            Some(a) => a,
            None => {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "The encryption key could not be decapsulated from the initialization vector."));
            }
        };
        ek.decrypt(iv, data)
    }

    pub fn size(&self) -> KeySize {
        match &self {
            PrivateEncryptKey::Ntru128 { pk: _, sk: _ } => KeySize::Bit128,
            PrivateEncryptKey::Ntru192 { pk: _, sk: _ } => KeySize::Bit192,
            PrivateEncryptKey::Ntru256 { pk: _, sk: _ } => KeySize::Bit256,
        }
    }
}

impl std::fmt::Display
for PrivateEncryptKey
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrivateEncryptKey::Ntru128 { pk: _, sk: _ } => write!(f, "ntru128:pk:{}+sk", self.hash()),
            PrivateEncryptKey::Ntru192 { pk: _, sk: _ } => write!(f, "ntru192:pk:{}+sk", self.hash()),
            PrivateEncryptKey::Ntru256 { pk: _, sk: _ } => write!(f, "ntru256:pk:{}+sk", self.hash()),
        }
    }
}

/// Public encryption keys provide the ability to encrypt a secret
/// without the ability to decrypt it yourself - this capability is
/// useful for key-exchange and trust validation in the crypto chain.
/// Asymetric crypto in ATE uses the leading candidates from NIST
/// that provide protection against quantom computer attacks
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub enum PublicEncryptKey {
    Ntru128 {
        pk: Vec<u8>,
    },
    Ntru192 {
        pk: Vec<u8>,
    },
    Ntru256 {
        pk: Vec<u8>,
    }
}

impl PublicEncryptKey
{
    pub fn from_bytes(bytes: Vec<u8>) -> Option<PublicEncryptKey>
    {
        match bytes.len() {
            a if a == ntru128::public_key_bytes() => Some(PublicEncryptKey::Ntru128 { pk: bytes }),
            a if a == ntru192::public_key_bytes() => Some(PublicEncryptKey::Ntru192 { pk: bytes }),
            a if a == ntru256::public_key_bytes() => Some(PublicEncryptKey::Ntru256 { pk: bytes }),
            _ => None,
        }
    }

    pub fn pk(&self) -> Vec<u8> { 
        match &self {
            PublicEncryptKey::Ntru128 { pk } => pk.clone(),
            PublicEncryptKey::Ntru192 { pk } => pk.clone(),
            PublicEncryptKey::Ntru256 { pk } => pk.clone(),
        }
    }

    #[allow(dead_code)]
    pub fn hash(&self) -> Hash {
        match &self {
            PublicEncryptKey::Ntru128 { pk } => Hash::from_bytes(&pk[..]),
            PublicEncryptKey::Ntru192 { pk } => Hash::from_bytes(&pk[..]),
            PublicEncryptKey::Ntru256 { pk } => Hash::from_bytes(&pk[..]),
        }
    }

    #[allow(dead_code)]
    pub fn encapsulate(&self) -> (InitializationVector, EncryptKey) {
        match &self {
            PublicEncryptKey::Ntru128 { pk } => {
                let pk = ntru128::PublicKey::from_bytes(&pk[..]).unwrap();
                let (ss, ct) = ntru128::encapsulate(&pk);
                let iv = InitializationVector::from_bytes(Vec::from(ct.as_bytes()));
                (iv, EncryptKey::from_seed_bytes(ss.as_bytes(), KeySize::Bit128))
            },
            PublicEncryptKey::Ntru192 { pk } => {
                let pk = ntru192::PublicKey::from_bytes(&pk[..]).unwrap();
                let (ss, ct) = ntru192::encapsulate(&pk);
                let iv = InitializationVector::from_bytes(Vec::from(ct.as_bytes()));
                (iv, EncryptKey::from_seed_bytes(ss.as_bytes(), KeySize::Bit192))
            },
            PublicEncryptKey::Ntru256 { pk } => {
                let pk = ntru256::PublicKey::from_bytes(&pk[..]).unwrap();
                let (ss, ct) = ntru256::encapsulate(&pk);
                let iv = InitializationVector::from_bytes(Vec::from(ct.as_bytes()));
                (iv, EncryptKey::from_seed_bytes(ss.as_bytes(), KeySize::Bit256))
            },
        }
    }

    pub fn encrypt(&self, data: &[u8]) -> Result<EncryptResult, std::io::Error> {
        let (iv, ek) = self.encapsulate();
        let data = ek.encrypt_with_iv(&iv, data)?;
        Ok(
            EncryptResult {
                iv,
                data,
            }
        )
    }

    pub fn size(&self) -> KeySize {
        match &self {
            PublicEncryptKey::Ntru128 { pk: _ } => KeySize::Bit128,
            PublicEncryptKey::Ntru192 { pk: _ } => KeySize::Bit192,
            PublicEncryptKey::Ntru256 { pk: _ } => KeySize::Bit256,
        }
    }
}

impl std::fmt::Display
for PublicEncryptKey
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PublicEncryptKey::Ntru128 { pk: _ } => write!(f, "ntru128:pk:{}", self.hash()),
            PublicEncryptKey::Ntru192 { pk: _ } => write!(f, "ntru192:pk:{}", self.hash()),
            PublicEncryptKey::Ntru256 { pk: _ } => write!(f, "ntru256:pk:{}", self.hash()),
        }
    }
}

#[test]
fn test_secure_random() {
    crate::utils::bootstrap_env();

    let t = 1024;
    for _ in 0..t {
        let mut data = [0 as u8; 1024];
        RandomGeneratorAccessor::default().fill_bytes(&mut data);
    }
}

#[allow(deprecated)]
#[test]
fn test_encrypt_key_seeding_old() {
    crate::utils::bootstrap_env();

    let provided = EncryptKey::from_string("test".to_string(), KeySize::Bit256);
    let expected = EncryptKey::Aes256([109, 23, 234, 219, 133, 97, 152, 126, 236, 229, 197, 134, 107, 89, 217, 82, 107, 27, 70, 176, 239, 71, 218, 171, 68, 75, 54, 215, 249, 219, 231, 69]);
    assert_eq!(provided, expected);

    let provided = EncryptKey::from_string("test2".to_string(), KeySize::Bit256);
    let expected = EncryptKey::Aes256([230, 248, 163, 17, 228, 69, 199, 43, 44, 106, 137, 243, 229, 187, 80, 173, 250, 183, 169, 165, 247, 153, 250, 8, 248, 187, 48, 83, 165, 91, 255, 180]);
    assert_eq!(provided, expected);
}

#[test]
fn test_encrypt_key_seeding_new() {
    crate::utils::bootstrap_env();

    let provided = EncryptKey::from_seed_string("test".to_string(), KeySize::Bit256);
    let expected = EncryptKey::Aes256([83, 208, 186, 19, 115, 7, 212, 194, 249, 182, 103, 76, 131, 237, 189, 88, 183, 12, 15, 67, 64, 19, 62, 208, 173, 198, 251, 161, 210, 71, 138, 106]);
    assert_eq!(provided, expected);

    let provided = EncryptKey::from_seed_string("test2".to_string(), KeySize::Bit256);
    let expected = EncryptKey::Aes256([159, 117, 193, 157, 58, 233, 178, 104, 76, 27, 193, 46, 126, 60, 139, 195, 55, 116, 66, 157, 228, 23, 223, 83, 106, 242, 81, 107, 17, 200, 1, 157]);
    assert_eq!(provided, expected);
}

#[test]
fn test_asym_crypto_128()
{
    crate::utils::bootstrap_env();

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
    crate::utils::bootstrap_env();

    let key = EncryptKey::generate(KeySize::Bit256);
    let private = EncryptedPrivateKey::generate(&key).unwrap();
    let public = private.as_public_key();

    let plain = b"test";
    let sig = private.as_private_key(&key).unwrap().sign(plain).unwrap();
    assert!(public.verify(plain, &sig[..]).unwrap(), "Signature verificaton failed");

    let negative = b"blahtest";
    assert!(public.verify(negative, &sig[..]).unwrap() == false, "Signature verificaton passes when it should not");
}

#[test]
fn test_ntru_encapsulate() -> Result<(), AteError>
{
    crate::utils::bootstrap_env();

    static KEY_SIZES: [KeySize; 3] = [KeySize::Bit128, KeySize::Bit192, KeySize::Bit256];
    for key_size in KEY_SIZES.iter() {
        let sk = PrivateEncryptKey::generate(key_size.clone());
        let pk = sk.as_public_key();
        let (iv, ek1) = pk.encapsulate();
        let ek2 = sk.decapsulate(&iv).unwrap();

        let plain_text1 = "the cat ran up the wall".to_string();
        let cipher_text = ek1.encrypt(plain_text1.as_bytes())?;
        let plain_test2 = String::from_utf8(ek2.decrypt(&cipher_text.iv, &cipher_text.data)?).unwrap();

        assert_eq!(plain_text1, plain_test2);
    }

    Ok(())
}

#[test]
fn test_ntru_encrypt() -> Result<(), AteError>
{
    crate::utils::bootstrap_env();
    
    static KEY_SIZES: [KeySize; 3] = [KeySize::Bit128, KeySize::Bit192, KeySize::Bit256];
    for key_size in KEY_SIZES.iter() {
        let sk = PrivateEncryptKey::generate(key_size.clone());
        let pk = sk.as_public_key();
        
        let plain_text1 = "the cat ran up the wall".to_string();
        let cipher_text = pk.encrypt(plain_text1.as_bytes())?;
        let plain_test2 = String::from_utf8(sk.decrypt(&cipher_text.iv, &cipher_text.data)?).unwrap();

        assert_eq!(plain_text1, plain_test2);
    }

    Ok(())
}

#[test]
fn test_multi_encrypt() -> Result<(), AteError>
{
    crate::utils::bootstrap_env();

    static KEY_SIZES: [KeySize; 3] = [KeySize::Bit128, KeySize::Bit192, KeySize::Bit256];
    for key_size in KEY_SIZES.iter() {
        let client1 = PrivateEncryptKey::generate(key_size.clone());
        let client2 = PrivateEncryptKey::generate(key_size.clone());
        let client3 = PrivateEncryptKey::generate(key_size.clone());
        
        let plain_text1 = "the cat ran up the wall".to_string();
        let mut multi = MultiEncryptedSecureData::new(&client1.as_public_key(), "meta".to_string(), plain_text1.clone())?;
        multi.add(&client2.as_public_key(), "another_meta".to_string(), &client1)?;

        let plain_text2 = multi.unwrap(&client1)?.expect("Should have decrypted.");
        assert_eq!(plain_text1, plain_text2);
        let plain_text2 = multi.unwrap(&client2)?.expect("Should have decrypted.");
        assert_eq!(plain_text1, plain_text2);
        let plain_text2 = multi.unwrap(&client3)?;
        assert!(plain_text2.is_none(), "The last client should not load anything");
    }

    Ok(())
}