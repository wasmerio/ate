#[allow(unused_imports)]
use log::{info, error, debug};
use rand::{RngCore};
use std::{cell::RefCell};
use std::result::Result;

use super::fast_random::*;
use super::*;

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

impl RandomGeneratorAccessor {
    pub fn generate_encrypt_key(size: KeySize) -> EncryptKey {
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
}