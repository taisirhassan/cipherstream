use std::error::Error;
use std::fmt;
use std::path::Path;
use ring::{
    aead::{self, AES_256_GCM, LessSafeKey, UnboundKey},
    digest::{self, SHA256},
    rand::{SystemRandom, SecureRandom},
    signature::{Ed25519KeyPair, KeyPair},
};
use tokio::fs::File;
use tokio::io::AsyncReadExt;

/// Cryptographic error types
#[derive(Debug)]
pub enum CryptoError {
    /// Error during encryption operations
    Encryption,
    /// Error during decryption operations
    Decryption,
    /// Invalid key provided
    InvalidKey,
    /// Error during signing operations
    Signing,
    /// Error during signature verification
    Verification,
    /// Error during key generation
    KeyGeneration,
    /// I/O error during file operations
    Io(std::io::Error),
    /// Other cryptographic errors
    Other(String),
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CryptoError::Encryption => write!(f, "Encryption failed"),
            CryptoError::Decryption => write!(f, "Decryption failed"),
            CryptoError::InvalidKey => write!(f, "Invalid key"),
            CryptoError::Signing => write!(f, "Signing failed"),
            CryptoError::Verification => write!(f, "Signature verification failed"),
            CryptoError::KeyGeneration => write!(f, "Key generation failed"),
            CryptoError::Io(err) => write!(f, "I/O error: {}", err),
            CryptoError::Other(msg) => write!(f, "Crypto error: {}", msg),
        }
    }
}

impl Error for CryptoError {}

impl From<std::io::Error> for CryptoError {
    fn from(err: std::io::Error) -> Self {
        CryptoError::Io(err)
    }
}

pub type CryptoResult<T> = Result<T, CryptoError>;

/// Generate a random AES-256 key
pub fn generate_key() -> CryptoResult<Vec<u8>> {
    let mut key = vec![0u8; 32]; // 256 bits
    let rng = SystemRandom::new();
    rng.fill(&mut key).map_err(|_| CryptoError::KeyGeneration)?;
    Ok(key)
}

/// Encrypt data with AES-256-GCM
pub fn encrypt(data: &[u8], key: &[u8]) -> CryptoResult<Vec<u8>> {
    // Generate a random nonce
    let mut nonce_bytes = [0u8; 12];
    let rng = SystemRandom::new();
    rng.fill(&mut nonce_bytes).map_err(|_| CryptoError::Encryption)?;
    
    let unbound_key = UnboundKey::new(&AES_256_GCM, key).map_err(|_| CryptoError::InvalidKey)?;
    let aead_key = LessSafeKey::new(unbound_key);
    let nonce = aead::Nonce::assume_unique_for_key(nonce_bytes);
    
    let mut in_out = data.to_vec();
    aead_key.seal_in_place_append_tag(nonce, aead::Aad::empty(), &mut in_out)
        .map_err(|_| CryptoError::Encryption)?;
    
    // Prepend nonce to encrypted data
    let mut result = nonce_bytes.to_vec();
    result.extend_from_slice(&in_out);
    
    Ok(result)
}

/// Decrypt data encrypted with AES-256-GCM
pub fn decrypt(encrypted: &[u8], key: &[u8]) -> CryptoResult<Vec<u8>> {
    // Expect nonce (12) + tag (16)
    if encrypted.len() < 12 + 16 {
        return Err(CryptoError::Decryption);
    }
    let (nonce_bytes, ciphertext_and_tag) = encrypted.split_at(12);

    let unbound = UnboundKey::new(&AES_256_GCM, key)
        .map_err(|_| CryptoError::InvalidKey)?;
    let aead_key = LessSafeKey::new(unbound);

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
pub fn generate_signing_keypair() -> CryptoResult<(Vec<u8>, Vec<u8>)> {
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
pub fn sign_message(message: &[u8], private_key: &[u8]) -> CryptoResult<Vec<u8>> {
    let key_pair = Ed25519KeyPair::from_pkcs8(private_key)
        .map_err(|_| CryptoError::InvalidKey)?;
    let signature = key_pair.sign(message);
    Ok(signature.as_ref().to_vec())
}

/// Verify a signature using an Ed25519 public key
pub fn verify_signature(message: &[u8], signature: &[u8], public_key: &[u8]) -> CryptoResult<bool> {
    use ring::signature::{UnparsedPublicKey, ED25519};
    
    let public_key = UnparsedPublicKey::new(&ED25519, public_key);
    match public_key.verify(message, signature) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Compute the SHA-256 hash of a file
pub async fn compute_file_hash<P: AsRef<Path>>(path: P) -> CryptoResult<String> {
    let mut file = File::open(path).await?;
    let mut hasher = digest::Context::new(&SHA256);
    let mut buffer = [0; 8192];
    
    loop {
        let bytes_read = file.read(&mut buffer).await?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    
    let digest = hasher.finish();
    Ok(hex::encode(digest.as_ref()))
}

/// Hash submodule for file hashing operations
pub mod hash {
    use super::*;
    
    /// Compute the SHA-256 hash of a file (alias for the main function)
    pub async fn compute_file_hash<P: AsRef<Path>>(path: P) -> CryptoResult<String> {
        super::compute_file_hash(path).await
    }
    
    /// Compute the SHA-256 hash of data in memory
    pub fn compute_data_hash(data: &[u8]) -> String {
        let digest = digest::digest(&SHA256, data);
        hex::encode(digest.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_key_generation() {
        let key = generate_key().unwrap();
        assert_eq!(key.len(), 32);
        
        // Generate another key and ensure they're different
        let key2 = generate_key().unwrap();
        assert_ne!(key, key2);
    }

    #[test]
    fn test_encrypt_decrypt() {
        let key = generate_key().unwrap();
        let data = b"Hello, world!";
        
        let encrypted = encrypt(data, &key).unwrap();
        assert_ne!(encrypted, data);
        
        let decrypted = decrypt(&encrypted, &key).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = generate_key().unwrap();
        let key2 = generate_key().unwrap();
        let data = b"Secret data";
        
        let encrypted = encrypt(data, &key1).unwrap();
        let result = decrypt(&encrypted, &key2);
        assert!(result.is_err());
    }

    #[test]
    fn test_signing_verification() {
        let (private_key, public_key) = generate_signing_keypair().unwrap();
        let message = b"Test message";
        
        let signature = sign_message(message, &private_key).unwrap();
        let verified = verify_signature(message, &signature, &public_key).unwrap();
        assert!(verified);
        
        // Test with wrong message
        let wrong_message = b"Wrong message";
        let verified = verify_signature(wrong_message, &signature, &public_key).unwrap();
        assert!(!verified);
    }

    #[tokio::test]
    async fn test_file_hash() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Test file content";
        temp_file.write_all(test_data).unwrap();
        
        let hash = compute_file_hash(temp_file.path()).await.unwrap();
        assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex characters
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }
} 