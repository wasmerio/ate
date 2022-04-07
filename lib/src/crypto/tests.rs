#![cfg(test)]
use crate::error::*;
use std::result::Result;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use serde::{Serialize, Deserialize};

use rand::RngCore;

use super::*;

#[test]
fn test_secure_random() {
    crate::utils::bootstrap_test_env();

    let t = 1024;
    for _ in 0..t {
        let mut data = [0 as u8; 1024];
        RandomGeneratorAccessor::default().fill_bytes(&mut data);
    }
}

#[test]
fn test_encrypt_key_seeding_new() {
    crate::utils::bootstrap_test_env();

    let provided = EncryptKey::from_seed_string("test".to_string(), KeySize::Bit256);
    let expected = EncryptKey::Aes256([
        83, 208, 186, 19, 115, 7, 212, 194, 249, 182, 103, 76, 131, 237, 189, 88, 183, 12, 15, 67,
        64, 19, 62, 208, 173, 198, 251, 161, 210, 71, 138, 106,
    ]);
    assert_eq!(provided, expected);

    let provided = EncryptKey::from_seed_string("test2".to_string(), KeySize::Bit256);
    let expected = EncryptKey::Aes256([
        159, 117, 193, 157, 58, 233, 178, 104, 76, 27, 193, 46, 126, 60, 139, 195, 55, 116, 66,
        157, 228, 23, 223, 83, 106, 242, 81, 107, 17, 200, 1, 157,
    ]);
    assert_eq!(provided, expected);
}

#[test]
fn test_asym_crypto_128() {
    crate::utils::bootstrap_test_env();

    let key = EncryptKey::generate(KeySize::Bit128);
    let private = EncryptedPrivateKey::generate(&key);
    let public = private.as_public_key();

    let plain = b"test";
    let sig = private.as_private_key(&key).sign(plain).unwrap();
    assert!(
        public.verify(plain, &sig[..]).unwrap(),
        "Signature verificaton failed"
    );

    let negative = b"blahtest";
    assert!(
        public.verify(negative, &sig[..]).unwrap() == false,
        "Signature verificaton passes when it should not"
    );
}

#[test]
fn test_asym_crypto_256() {
    crate::utils::bootstrap_test_env();

    let key = EncryptKey::generate(KeySize::Bit256);
    let private = EncryptedPrivateKey::generate(&key);
    let public = private.as_public_key();

    let plain = b"test";
    let sig = private.as_private_key(&key).sign(plain).unwrap();
    assert!(
        public.verify(plain, &sig[..]).unwrap(),
        "Signature verificaton failed"
    );

    let negative = b"blahtest";
    assert!(
        public.verify(negative, &sig[..]).unwrap() == false,
        "Signature verificaton passes when it should not"
    );
}

#[test]
fn test_ntru_encapsulate() -> Result<(), AteError> {
    crate::utils::bootstrap_test_env();

    static KEY_SIZES: [KeySize; 3] = [KeySize::Bit128, KeySize::Bit192, KeySize::Bit256];
    for key_size in KEY_SIZES.iter() {
        let sk = PrivateEncryptKey::generate(key_size.clone());
        let pk = sk.as_public_key();
        let (iv, ek1) = pk.encapsulate();
        let ek2 = sk.decapsulate(&iv).unwrap();

        assert_eq!(ek1.hash(), ek2.hash());

        let plain_text1 = "the cat ran up the wall".to_string();
        let cipher_text = ek1.encrypt(plain_text1.as_bytes());
        let plain_test2 =
            String::from_utf8(ek2.decrypt(&cipher_text.iv, &cipher_text.data)).unwrap();

        assert_eq!(plain_text1, plain_test2);
    }

    Ok(())
}

#[test]
fn test_ntru_encrypt() -> Result<(), AteError> {
    crate::utils::bootstrap_test_env();

    static KEY_SIZES: [KeySize; 3] = [KeySize::Bit128, KeySize::Bit192, KeySize::Bit256];
    for key_size in KEY_SIZES.iter() {
        let sk = PrivateEncryptKey::generate(key_size.clone());
        let pk = sk.as_public_key();

        let plain_text1 = "the cat ran up the wall".to_string();
        let cipher_text = pk.encrypt(plain_text1.as_bytes());
        let plain_test2 =
            String::from_utf8(sk.decrypt(&cipher_text.iv, &cipher_text.data)?).unwrap();

        assert_eq!(plain_text1, plain_test2);
    }

    Ok(())
}

#[test]
fn test_derived_keys() -> Result<(), AteError> {
    static KEY_SIZES: [KeySize; 3] = [KeySize::Bit128, KeySize::Bit192, KeySize::Bit256];
    for key_size1 in KEY_SIZES.iter() {
        for key_size2 in KEY_SIZES.iter() {
            // Generate a derived key and encryption key
            let key2 = EncryptKey::generate(*key_size1);
            let mut key1 = DerivedEncryptKey::new(&key2);

            // Encrypt some data
            let plain_text1 = "the cat ran up the wall".to_string();
            let encrypted_text1 = key1.transmute(&key2)?.encrypt(plain_text1.as_bytes());

            // Check that it decrypts properly
            let plain_text2 = String::from_utf8(
                key1.transmute(&key2)?
                    .decrypt(&encrypted_text1.iv, &encrypted_text1.data[..]),
            )
            .unwrap();
            assert_eq!(plain_text1, plain_text2);

            // Now change the key
            let key3 = EncryptKey::generate(*key_size2);
            key1.change(&key2, &key3)?;

            // The decryption with the old key which should now fail
            let plain_text2 = match String::from_utf8(
                key1.transmute(&key2)?
                    .decrypt(&encrypted_text1.iv, &encrypted_text1.data[..]),
            ) {
                Ok(a) => a,
                Err(_) => "nothing".to_string(),
            };
            assert_ne!(plain_text1, plain_text2);

            // Check that it decrypts properly with the new key
            let plain_text2 = String::from_utf8(
                key1.transmute(&key3)?
                    .decrypt(&encrypted_text1.iv, &encrypted_text1.data[..]),
            )
            .unwrap();
            assert_eq!(plain_text1, plain_text2);
        }
    }

    Ok(())
}

#[test]
fn test_public_secure_data() -> Result<(), AteError> {
    crate::utils::bootstrap_test_env();

    #[derive(Debug, Serialize, Deserialize, Clone)]
    struct TestClass {
        data: String,
    }

    static KEY_SIZES: [KeySize; 3] = [KeySize::Bit128, KeySize::Bit192, KeySize::Bit256];
    for key_size in KEY_SIZES.iter() {
        let key = PrivateEncryptKey::generate(key_size.clone());
        let container = PublicEncryptedSecureData::<TestClass>::new(key.as_public_key(), TestClass {
            data: "the cat ran up the wall".to_string()
        }).unwrap();

        let out = container.unwrap(&key).unwrap();
        assert_eq!(out.data.as_str(), "the cat ran up the wall");
    }

    Ok(())
}

#[test]
fn test_secure_data() -> Result<(), AteError> {
    crate::utils::bootstrap_test_env();

    static KEY_SIZES: [KeySize; 3] = [KeySize::Bit128, KeySize::Bit192, KeySize::Bit256];
    for key_size in KEY_SIZES.iter() {
        let client1 = EncryptKey::generate(key_size.clone());

        let plain_text1 = "the cat ran up the wall".to_string();
        let cipher = EncryptedSecureData::new(&client1, plain_text1.clone())?;

        let plain_text2 = cipher.unwrap(&client1).expect("Should have decrypted.");
        assert_eq!(plain_text1, plain_text2);
    }

    Ok(())
}

#[test]
fn test_multi_encrypt() -> Result<(), AteError> {
    crate::utils::bootstrap_test_env();

    static KEY_SIZES: [KeySize; 3] = [KeySize::Bit128, KeySize::Bit192, KeySize::Bit256];
    for key_size in KEY_SIZES.iter() {
        let client1 = PrivateEncryptKey::generate(key_size.clone());
        let client2 = PrivateEncryptKey::generate(key_size.clone());
        let client3 = PrivateEncryptKey::generate(key_size.clone());

        let plain_text1 = "the cat ran up the wall".to_string();
        let mut multi = MultiEncryptedSecureData::new(
            &client1.as_public_key(),
            "meta".to_string(),
            plain_text1.clone(),
        )?;
        multi.add(
            &client2.as_public_key(),
            "another_meta".to_string(),
            &client1,
        )?;

        let plain_text2 = multi.unwrap(&client1)?.expect("Should have decrypted.");
        assert_eq!(plain_text1, plain_text2);
        let plain_text2 = multi.unwrap(&client2)?.expect("Should have decrypted.");
        assert_eq!(plain_text1, plain_text2);
        let plain_text2 = multi.unwrap(&client3)?;
        assert!(
            plain_text2.is_none(),
            "The last client should not load anything"
        );
    }

    Ok(())
}

#[test]
fn test_signed_protected_data() -> Result<(), AteError> {
    let sign_key = PrivateSignKey::generate(KeySize::Bit256);
    let data = "test data".to_string();

    let test = SignedProtectedData::new(&sign_key, data)?;
    assert!(
        test.verify(&sign_key.as_public_key())?,
        "Failed to verify the protected data"
    );

    Ok(())
}
