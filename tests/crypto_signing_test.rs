use cipherstream::crypto;

#[test]
fn test_generate_signing_keypair() {
    // Generate a signing keypair
    let (private_key, public_key) = crypto::generate_signing_keypair().unwrap();

    // Verify that keys are not empty
    assert!(!private_key.is_empty());
    assert!(!public_key.is_empty());

    // Ed25519 public keys should be 32 bytes
    assert_eq!(public_key.len(), 32);

    // PKCS#8 private keys are longer and include the public key
    assert!(private_key.len() > 32);

    // Generate another keypair and make sure they're different
    let (private_key2, public_key2) = crypto::generate_signing_keypair().unwrap();
    assert_ne!(private_key, private_key2);
    assert_ne!(public_key, public_key2);
}

#[test]
fn test_sign_and_verify_message() {
    // Generate a keypair
    let (private_key, public_key) = crypto::generate_signing_keypair().unwrap();

    // Create a test message
    let message = b"This is a test message to be signed";

    // Sign the message
    let signature = crypto::sign_message(message, &private_key).unwrap();

    // Verify the signature
    let result = crypto::verify_signature(message, &signature, &public_key).unwrap();
    assert!(result, "Signature verification failed");

    // Verify that modifying the message invalidates the signature
    let modified_message = b"This is a modified test message";
    let result = crypto::verify_signature(modified_message, &signature, &public_key).unwrap();
    assert!(
        !result,
        "Signature verification should fail with modified message"
    );

    // Generate a different keypair
    let (_, different_public_key) = crypto::generate_signing_keypair().unwrap();

    // Verify that using a different public key invalidates the signature
    let result = crypto::verify_signature(message, &signature, &different_public_key).unwrap();
    assert!(
        !result,
        "Signature verification should fail with wrong public key"
    );

    // Verify that modifying the signature invalidates it
    let mut modified_signature = signature.clone();
    if !modified_signature.is_empty() {
        modified_signature[0] = modified_signature[0].wrapping_add(1);
    }
    let result = crypto::verify_signature(message, &modified_signature, &public_key).unwrap();
    assert!(
        !result,
        "Signature verification should fail with modified signature"
    );
}

#[test]
fn test_sign_empty_message() {
    // Generate a keypair
    let (private_key, public_key) = crypto::generate_signing_keypair().unwrap();

    // Create an empty message
    let empty_message = b"";

    // Sign the empty message
    let signature = crypto::sign_message(empty_message, &private_key).unwrap();

    // Verify the signature for the empty message
    let result = crypto::verify_signature(empty_message, &signature, &public_key).unwrap();
    assert!(result, "Empty message signature verification failed");

    // Make sure it doesn't verify for a non-empty message
    let non_empty_message = b"Not empty";
    let result = crypto::verify_signature(non_empty_message, &signature, &public_key).unwrap();
    assert!(
        !result,
        "Signature verification should fail with different message"
    );
}

#[test]
fn test_sign_large_message() {
    // Generate a keypair
    let (private_key, public_key) = crypto::generate_signing_keypair().unwrap();

    // Create a large message (1MB)
    let large_message = vec![0x42; 1024 * 1024];

    // Sign the large message
    let signature = crypto::sign_message(&large_message, &private_key).unwrap();

    // Verify the signature for the large message
    let result = crypto::verify_signature(&large_message, &signature, &public_key).unwrap();
    assert!(result, "Large message signature verification failed");

    // Modify one byte in the large message
    let mut modified_large_message = large_message.clone();
    modified_large_message[1000] = 0x43;

    // Verify that the signature fails for the modified message
    let result =
        crypto::verify_signature(&modified_large_message, &signature, &public_key).unwrap();
    assert!(
        !result,
        "Signature verification should fail with modified large message"
    );
}

#[test]
fn test_invalid_private_key() {
    // Create an invalid private key
    let invalid_private_key = vec![0; 32]; // Not a valid PKCS#8 key

    // Attempt to sign with the invalid key
    let message = b"Test message";
    let result = crypto::sign_message(message, &invalid_private_key);

    // Verify that signing with an invalid key fails
    assert!(result.is_err());
}

#[test]
fn test_invalid_signature_length() {
    // Generate a keypair
    let (_, public_key) = crypto::generate_signing_keypair().unwrap();

    // Create an invalid signature (wrong length)
    let invalid_signature = vec![0; 32]; // Ed25519 signatures are 64 bytes

    // Attempt to verify with the invalid signature
    let message = b"Test message";
    let result = crypto::verify_signature(message, &invalid_signature, &public_key);

    // This should return false rather than error, as the implementation handles
    // invalid signatures gracefully
    assert!(!result.unwrap());
}
