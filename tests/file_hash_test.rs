use cipherstream::crypto;
use std::io::Write;
use tempfile::NamedTempFile;

#[tokio::test]
async fn test_compute_file_hash() {
    // Create a temporary file with known content
    let mut temp_file = NamedTempFile::new().unwrap();
    let test_content = b"This is a test file for hashing";
    temp_file.write_all(test_content).unwrap();

    // Compute the hash using our function
    let hash = crypto::compute_file_hash(temp_file.path())
        .await
        .expect("Failed to compute file hash");

    // Verify the hash is not empty and has the expected format for SHA-256 (64 hex chars)
    assert_eq!(hash.len(), 64);
    // Verify all characters are valid hex characters
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));

    // Create a different file
    let mut temp_file2 = NamedTempFile::new().unwrap();
    let different_content = b"This is a different file content";
    temp_file2.write_all(different_content).unwrap();

    // Compute the hash of the second file
    let hash2 = crypto::compute_file_hash(temp_file2.path())
        .await
        .expect("Failed to compute second file hash");

    // Verify that different content produces different hash
    assert_ne!(hash, hash2);

    // Create a file with the same content as the first file
    let mut temp_file3 = NamedTempFile::new().unwrap();
    temp_file3.write_all(test_content).unwrap();

    // Compute the hash of the third file
    let hash3 = crypto::compute_file_hash(temp_file3.path())
        .await
        .expect("Failed to compute third file hash");

    // Verify that same content produces the same hash (hash collision resistance is not tested here)
    assert_eq!(hash, hash3);
}

#[tokio::test]
async fn test_hash_empty_file() {
    // Create an empty temporary file
    let temp_file = NamedTempFile::new().unwrap();

    // Compute the hash of the empty file
    let hash = crypto::compute_file_hash(temp_file.path())
        .await
        .expect("Failed to compute hash of empty file");

    // Known SHA-256 hash of empty string/file
    // e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
    let expected_empty_hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    // Verify the hash matches the expected value
    assert_eq!(hash.to_lowercase(), expected_empty_hash);
}

#[tokio::test]
async fn test_hash_module_function() {
    // Create a temporary file with known content
    let mut temp_file = NamedTempFile::new().unwrap();
    let test_content = b"Testing the hash module function";
    temp_file.write_all(test_content).unwrap();

    // Compute the hash using the module function
    let hash = crypto::hash::compute_file_hash(temp_file.path())
        .await
        .expect("Failed to compute file hash using module function");

    // Compute the hash using the main function for comparison
    let main_hash = crypto::compute_file_hash(temp_file.path())
        .await
        .expect("Failed to compute file hash using main function");

    // The hashes should be identical since they use the same algorithm
    assert_eq!(hash, main_hash);
}

#[tokio::test]
async fn test_nonexistent_file_hash() {
    // Try to compute hash of a non-existent file
    let result = crypto::compute_file_hash("/nonexistent/file/path.txt").await;

    // Verify that the operation fails with an error
    assert!(result.is_err());
}

#[tokio::test]
async fn test_hash_large_file() {
    // Create a temporary file with 1MB of data
    let mut temp_file = NamedTempFile::new().unwrap();
    let large_data = vec![0x55; 1024 * 1024]; // 1MB of 0x55 bytes
    temp_file.write_all(&large_data).unwrap();

    // Compute the hash
    let hash = crypto::compute_file_hash(temp_file.path())
        .await
        .expect("Failed to compute hash of large file");

    // Verify the hash is still a valid SHA-256 hash
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
}
