#[allow(unused_imports)]
use log::{info, error, debug};
use rand::{RngCore, SeedableRng, rngs::adapter::ReseedingRng};
use rand_chacha::{ChaCha20Core, ChaCha20Rng};
use std::sync::{Mutex, MutexGuard};
use once_cell::sync::Lazy;
use std::result::Result;

static GLOBAL_SECURE_AND_FAST_RANDOM: Lazy<Mutex<ChaCha20Rng>> = Lazy::new(|| {
    Mutex::new(ChaCha20Rng::from_entropy())
});

#[derive(Default)]
pub(super) struct SingleThreadedSecureAndFastRandom {
}

impl<'a> SingleThreadedSecureAndFastRandom {
    pub(super) fn lock(&'a mut self) -> MutexGuard<'static, ChaCha20Rng> {
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

pub(super) struct SecureAndFastRandom
{
    pub(super) rng: Box<dyn RngCore>,
}

impl SecureAndFastRandom {
    pub(super) fn new() -> SecureAndFastRandom {
        let mut seeder = SingleThreadedSecureAndFastRandom::default();
        let rng = ChaCha20Core::from_rng(&mut seeder).expect("Failed to properly seed the secure random number generator");
        let reseeding_rng = ReseedingRng::new(rng, 0, seeder);
        SecureAndFastRandom {
            rng: Box::new(reseeding_rng),
        }
    }
}