use ring::{
    aead::{self, Aad, BoundKey, Nonce, NonceSequence, SealingKey, UnboundKey, AES_256_GCM},
    rand::{SecureRandom, SystemRandom},
    signature::{self, Ed25519KeyPair, KeyPair},
    digest,
};
use thiserror::Error;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

/// Compute SHA-256 hash for a file
pub async fn compute_file_hash<P: AsRef<Path>>(path: P) -> Result<String, CryptoError> {
    let mut file = File::open(path).await.map_err(|_| CryptoError::HashingError)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).await.map_err(|_| CryptoError::HashingError)?;
    
    let hash = digest::digest(&digest::SHA256, &buffer);
    Ok(hex::encode(hash.as_ref()))
}

/// Module-specific hash functions
pub mod hash {
    use super::*;
    
    /// Compute hash for a file
    pub async fn compute_file_hash<P: AsRef<Path>>(path: P) -> Result<String, std::io::Error> {
        let mut file = File::open(path).await?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;
        
        let hash = digest::digest(&digest::SHA256, &buffer);
        Ok(hex::encode(hash.as_ref()))
    }
}

/// Crypto operation errors
#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Failed to generate key")]
    KeyGeneration,
    
    #[error("Invalid key material")]
    InvalidKey,
    
    #[error("Encryption failed")]
    Encryption,
    
    #[error("Decryption failed")]
    Decryption,
    
    #[error("Signature generation failed")]
    SigningError,
    
    #[error("Signature verification failed")]
    VerificationError,
    
    #[error("Hashing operation failed")]
    HashingError,
}

/// Generate a random AES-256 key
pub fn generate_key() -> Result<Vec<u8>, CryptoError> {
    let mut key = vec![0u8; 32]; // 256 bits
    let rng = SystemRandom::new();
    rng.fill(&mut key).map_err(|_| CryptoError::KeyGeneration)?;
    Ok(key)
}

/// Encrypt data with AES-256-GCM
pub fn encrypt(data: &[u8], key: &[u8]) -> Result<Vec<u8>, CryptoError> {
    // Create nonce and prepend it to the output
    let mut nonce_bytes = [0u8; 12];
    let rng = SystemRandom::new();
    rng.fill(&mut nonce_bytes).map_err(|_| CryptoError::Encryption)?;

    // Setup the encryption
    let unbound_key = UnboundKey::new(&AES_256_GCM, key).map_err(|_| CryptoError::InvalidKey)?;
    let nonce_sequence = FixedNonce::new(nonce_bytes);
    let mut sealing_key = SealingKey::new(unbound_key, nonce_sequence);

    // Encrypt the data
    let aad = Aad::empty();
    let mut in_out = data.to_vec();
    sealing_key.seal_in_place_append_tag(aad, &mut in_out).map_err(|_| CryptoError::Encryption)?;

    // Prepend the nonce
    let mut result = nonce_bytes.to_vec();
    result.extend_from_slice(&in_out);
    
    Ok(result)
}

/// Decrypt data encrypted with AES-256-GCM
pub fn decrypt(encrypted: &[u8], key: &[u8]) -> Result<Vec<u8>, CryptoError> {
    // Expect nonce (12) + tag (16)
    if encrypted.len() < 12 + 16 {
        return Err(CryptoError::Decryption);
    }
    let (nonce_bytes, ciphertext_and_tag) = encrypted.split_at(12);

    let unbound = aead::UnboundKey::new(&aead::AES_256_GCM, key)
        .map_err(|_| CryptoError::InvalidKey)?;
    let aead_key = aead::LessSafeKey::new(unbound);

    let mut nonce_arr = [0u8; 12];
    nonce_arr.copy_from_slice(nonce_bytes);
    let nonce = aead::Nonce::assume_unique_for_key(nonce_arr);

    let mut buffer = ciphertext_and_tag.to_vec();
    let decrypted = aead_key
        .open_in_place(nonce, aead::Aad::empty(), &mut buffer)
        .map_err(|_| CryptoError::Decryption)?;
    Ok(decrypted.to_vec())
}

/// Generate an Ed25519 signing keypair
pub fn generate_signing_keypair() -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
    let rng = SystemRandom::new();
    let pkcs8_bytes = Ed25519KeyPair::generate_pkcs8(&rng)
        .map_err(|_| CryptoError::KeyGeneration)?;
    let key_pair = Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref())
        .map_err(|_| CryptoError::InvalidKey)?;
    
    let private_key = pkcs8_bytes.as_ref().to_vec();
    let public_key = key_pair.public_key().as_ref().to_vec();
    
    Ok((private_key, public_key))
}

/// Sign a message using an Ed25519 private key
pub fn sign_message(message: &[u8], private_key: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let key_pair = Ed25519KeyPair::from_pkcs8(private_key)
        .map_err(|_| CryptoError::InvalidKey)?;
    let signature = key_pair.sign(message);
    Ok(signature.as_ref().to_vec())
}

/// Verify a signature using an Ed25519 public key
pub fn verify_signature(message: &[u8], signature: &[u8], public_key: &[u8]) -> Result<bool, CryptoError> {
    let peer_public_key = signature::UnparsedPublicKey::new(
        &signature::ED25519,
        public_key,
    );
    
    match peer_public_key.verify(message, signature) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

// A fixed nonce for AES-GCM
struct FixedNonce {
    nonce_bytes: [u8; 12],
}

impl FixedNonce {
    fn new(nonce_bytes: [u8; 12]) -> Self {
        Self { nonce_bytes }
    }
}

impl NonceSequence for FixedNonce {
    fn advance(&mut self) -> Result<Nonce, ring::error::Unspecified> {
        Ok(Nonce::assume_unique_for_key(self.nonce_bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation() {
        let key = generate_key().unwrap();
        assert_eq!(key.len(), 32); // AES-256 key should be 32 bytes
    }

    #[test]
    fn test_encrypt_decrypt() {
        let data = b"Hello, this is a test message for encryption!";
        let key = generate_key().unwrap();
        
        // Encrypt the data
        let encrypted = encrypt(data, &key).unwrap();
        
        // Verify encrypted data is different from original
        assert_ne!(&encrypted, data);
        
        // Decrypt the data
        let decrypted = decrypt(&encrypted, &key).unwrap();
        
        // Verify decrypted data matches original
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_wrong_key_fails() {
        let data = b"Secret message";
        let key1 = generate_key().unwrap();
        let key2 = generate_key().unwrap();
        
        // Encrypt with key1
        let encrypted = encrypt(data, &key1).unwrap();
        
        // Attempt to decrypt with key2 should fail
        let result = decrypt(&encrypted, &key2);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_signing_verification() {
        let message = b"Test message for signing";
        
        // Generate a keypair
        let (private_key, public_key) = generate_signing_keypair().unwrap();
        
        // Sign the message
        let signature = sign_message(message, &private_key).unwrap();
        
        // Verify the signature
        let valid = verify_signature(message, &signature, &public_key).unwrap();
        assert!(valid);
        
        // Try with a different message
        let different_message = b"Different message";
        let invalid = verify_signature(different_message, &signature, &public_key).unwrap();
        assert!(!invalid);
    }
    
    #[tokio::test]
    async fn test_file_hash() {
        // Create a temporary file
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test_hash.txt");
        
        // Write content to the file
        let content = b"Test content for hashing";
        tokio::fs::write(&file_path, content).await.unwrap();
        
        // Compute hash
        let hash = compute_file_hash(&file_path).await.unwrap();
        
        // Verify hash is not empty
        assert!(!hash.is_empty());
        
        // Verify hash is consistent
        let hash2 = compute_file_hash(&file_path).await.unwrap();
        assert_eq!(hash, hash2);
    }
} 