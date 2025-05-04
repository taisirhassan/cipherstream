use std::path::Path;
use std::fs;

// Import crypto functions from the main crate
use cipherstream::crypto;

#[tokio::test]
async fn test_file_encryption_decryption() {
    // Create a temporary test file
    let test_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let test_file_path = test_dir.path().join("test_file.txt");
    let encrypted_file_path = test_dir.path().join("encrypted_file.dat");
    let decrypted_file_path = test_dir.path().join("decrypted_file.txt");
    
    // Write test data to the file
    let test_data = "This is a test file for encryption and decryption testing.";
    fs::write(&test_file_path, test_data).expect("Failed to write test file");
    
    // Generate a key for encryption
    let key = crypto::generate_key().expect("Failed to generate encryption key");
    
    // Encrypt the file
    encrypt_file(&test_file_path, &encrypted_file_path, &key)
        .await
        .expect("Failed to encrypt file");
    
    // Verify encrypted file exists and is different from original
    assert!(encrypted_file_path.exists());
    let encrypted_content = fs::read(&encrypted_file_path).expect("Failed to read encrypted file");
    let original_content = fs::read(&test_file_path).expect("Failed to read original file");
    assert_ne!(encrypted_content, original_content);
    
    // Decrypt the file
    decrypt_file(&encrypted_file_path, &decrypted_file_path, &key)
        .await
        .expect("Failed to decrypt file");
    
    // Verify decrypted file matches original
    let decrypted_data = fs::read_to_string(&decrypted_file_path).expect("Failed to read decrypted file");
    assert_eq!(decrypted_data, test_data);
}

// Helper function to encrypt a file
async fn encrypt_file<P: AsRef<Path>>(
    input_path: P,
    output_path: P,
    key: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    // Read the file content
    let file_content = tokio::fs::read(&input_path).await?;
    
    // Encrypt the content
    let encrypted = crypto::encrypt(&file_content, key)?;
    
    // Write encrypted content to output file
    tokio::fs::write(&output_path, encrypted).await?;
    
    Ok(())
}

// Helper function to decrypt a file
async fn decrypt_file<P: AsRef<Path>>(
    input_path: P,
    output_path: P,
    key: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    // Read the encrypted file content
    let encrypted_content = tokio::fs::read(&input_path).await?;
    
    // Decrypt the content
    let decrypted = crypto::decrypt(&encrypted_content, key)?;
    
    // Write decrypted content to output file
    tokio::fs::write(&output_path, decrypted).await?;
    
    Ok(())
} 