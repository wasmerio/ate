#![cfg(test)]
#[allow(unused_imports)]
use log::{info, error, debug};
use std::result::Result;
use crate::error::*;

use rand::{RngCore};

use super::*;

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
fn test_derived_keys() -> Result<(), AteError>
{
    static KEY_SIZES: [KeySize; 3] = [KeySize::Bit128, KeySize::Bit192, KeySize::Bit256];
    for key_size1 in KEY_SIZES.iter() {
        for key_size2 in KEY_SIZES.iter() {

            // Generate a derived key and encryption key
            let key2 = EncryptKey::generate(*key_size1);
            let mut key1 = DerivedEncryptKey::new(&key2)?;

            // Encrypt some data
            let plain_text1 = "the cat ran up the wall".to_string();
            let encrypted_text1 = key1.transmute(&key2)?.encrypt(plain_text1.as_bytes())?;

            // Check that it decrypts properly
            let plain_text2 = String::from_utf8(key1.transmute(&key2)?.decrypt(&encrypted_text1.iv, &encrypted_text1.data[..])?).unwrap();
            assert_eq!(plain_text1, plain_text2);

            // Now change the key
            let key3 = EncryptKey::generate(*key_size2);
            key1.change(&key2, &key3)?;

            // The decryption with the old key which should now fail
            let plain_text2 = match key1.transmute(&key2)?.decrypt(&encrypted_text1.iv, &encrypted_text1.data[..]) {
                Ok(a) => {
                    match String::from_utf8(a) {
                        Ok(a) => a,
                        Err(_) => "nothing".to_string()
                    }
                },
                Err(_) => "nothing".to_string()
            };
            assert_ne!(plain_text1, plain_text2);

            // Check that it decrypts properly with the new key
            let plain_text2 = String::from_utf8(key1.transmute(&key3)?.decrypt(&encrypted_text1.iv, &encrypted_text1.data[..])?).unwrap();
            assert_eq!(plain_text1, plain_text2);
        }
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