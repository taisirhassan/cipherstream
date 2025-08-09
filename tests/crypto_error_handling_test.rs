use cipherstream::crypto::{self, CryptoError};

#[test]
fn test_decrypt_wrong_key() {
    // Generate two different keys
    let key1 = crypto::generate_key().unwrap();
    let key2 = crypto::generate_key().unwrap();

    // Encrypt data with key1
    let data = b"Secret message for testing error handling";
    let encrypted = crypto::encrypt(data, &key1).unwrap();

    // Try to decrypt with key2 (should fail)
    let result = crypto::decrypt(&encrypted, &key2);
    assert!(result.is_err());

    // Ensure the error is the correct type
    match result {
        Err(CryptoError::Decryption) => {}
        _ => {
            panic!("Expected a decryption error but got a different result");
        }
    }
}

#[test]
fn test_decrypt_corrupted_data() {
    // Generate a key
    let key = crypto::generate_key().unwrap();

    // Encrypt data
    let data = b"This data will be corrupted";
    let mut encrypted = crypto::encrypt(data, &key).unwrap();

    // Corrupt the data (change a few bytes in the middle of the ciphertext)
    if encrypted.len() > 20 {
        encrypted[15] ^= 0xFF;
        encrypted[16] ^= 0xFF;
        encrypted[17] ^= 0xFF;
    }

    // Try to decrypt corrupted data (should fail)
    let result = crypto::decrypt(&encrypted, &key);
    assert!(result.is_err());

    // Check error type
    match result {
        Err(CryptoError::Decryption) => {}
        _ => {
            panic!("Expected a decryption error but got a different result");
        }
    }
}

#[test]
fn test_decrypt_truncated_data() {
    // Generate a key
    let key = crypto::generate_key().unwrap();

    // Encrypt data
    let data = b"This data will be truncated";
    let encrypted = crypto::encrypt(data, &key).unwrap();

    // Truncate the data (remove the authentication tag)
    let truncated = if encrypted.len() > 20 {
        encrypted[..encrypted.len() - 16].to_vec()
    } else {
        encrypted[..encrypted.len() / 2].to_vec()
    };

    // Try to decrypt truncated data (should fail)
    let result = crypto::decrypt(&truncated, &key);
    assert!(result.is_err());
}

#[test]
fn test_verify_signature_wrong_key() {
    // Generate two keypairs
    let (private_key1, _) = crypto::generate_signing_keypair().unwrap();
    let (_, public_key2) = crypto::generate_signing_keypair().unwrap();

    // Sign a message with private_key1
    let message = b"Test message for signature verification";
    let signature = crypto::sign_message(message, &private_key1).unwrap();

    // Verify with public_key2 (should fail)
    let result = crypto::verify_signature(message, &signature, &public_key2).unwrap();
    assert!(
        !result,
        "Signature verification should fail with wrong public key"
    );
}

#[test]
fn test_sign_with_invalid_key() {
    // Create an invalid private key
    let invalid_key = vec![0u8; 32]; // Not a valid PKCS#8 key

    // Try to sign with the invalid key
    let message = b"Test message";
    let result = crypto::sign_message(message, &invalid_key);

    // Should fail with InvalidKey error
    assert!(result.is_err());
    match result {
        Err(CryptoError::InvalidKey) => {}
        _ => {
            panic!("Expected an invalid key error but got a different result");
        }
    }
}

#[test]
fn test_decrypt_empty_data() {
    // Generate a key
    let key = crypto::generate_key().unwrap();

    // Try to decrypt empty data
    let result = crypto::decrypt(&[], &key);

    // Should fail
    assert!(result.is_err());
}

#[test]
fn test_decrypt_with_empty_key() {
    // Generate data to decrypt
    let key = crypto::generate_key().unwrap();
    let data = b"Test data";
    let encrypted = crypto::encrypt(data, &key).unwrap();

    // Try to decrypt with empty key
    let empty_key = vec![];
    let result = crypto::decrypt(&encrypted, &empty_key);

    // Should fail with InvalidKey error
    assert!(result.is_err());
    match result {
        Err(CryptoError::InvalidKey) => {}
        _ => {
            panic!("Expected an invalid key error but got a different result");
        }
    }
}
