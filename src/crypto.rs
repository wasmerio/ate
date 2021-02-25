extern crate rand;

use serde::{Serialize, Deserialize};
use rand::RngCore;
use rand::rngs::OsRng;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct EncryptKey {
    pub aes_key: [u8; 24],
}

impl EncryptKey {
    fn generate() -> EncryptKey {
        let mut rng = OsRng::default();
        let mut aes_key = [0; 24];
        rng.fill_bytes(&mut aes_key);
        
        EncryptKey {
            aes_key: aes_key,
        }
    }
}

impl Default for EncryptKey {
    fn default() -> EncryptKey {
        EncryptKey::generate()
    }
}