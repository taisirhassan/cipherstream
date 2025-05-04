use cipherstream::crypto;
use std::time::Instant;
use tempfile::NamedTempFile;
use std::io::Write;

#[test]
fn test_encryption_performance() {
    // Test with different data sizes
    let sizes = [
        1024,              // 1KB
        1024 * 10,         // 10KB
        1024 * 100,        // 100KB
        1024 * 1024,       // 1MB
    ];
    
    for &size in &sizes {
        // Create test data of specified size
        let data = vec![0x42; size];
        
        // Generate key
        let key = crypto::generate_key().unwrap();
        
        // Benchmark encryption
        let encrypt_start = Instant::now();
        let encrypted = crypto::encrypt(&data, &key).unwrap();
        let encrypt_duration = encrypt_start.elapsed();
        
        // Benchmark decryption
        let decrypt_start = Instant::now();
        let decrypted = crypto::decrypt(&encrypted, &key).unwrap();
        let decrypt_duration = decrypt_start.elapsed();
        
        // Verify that decryption gives back the original data
        assert_eq!(data, decrypted);
        
        // Output performance metrics
        println!(
            "Size: {:.2} KB | Encryption: {:.2?} | Decryption: {:.2?} | Throughput: {:.2} MB/s (enc) / {:.2} MB/s (dec)",
            size as f64 / 1024.0,
            encrypt_duration,
            decrypt_duration,
            (size as f64 / encrypt_duration.as_secs_f64()) / (1024.0 * 1024.0),
            (size as f64 / decrypt_duration.as_secs_f64()) / (1024.0 * 1024.0),
        );
    }
}

#[test]
fn test_signing_performance() {
    // Generate a signing keypair
    let (private_key, public_key) = crypto::generate_signing_keypair().unwrap();
    
    // Test with different message sizes
    let sizes = [
        1024,              // 1KB
        1024 * 10,         // 10KB
        1024 * 100,        // 100KB
        1024 * 1024,       // 1MB
    ];
    
    for &size in &sizes {
        // Create test data of specified size
        let data = vec![0x42; size];
        
        // Benchmark signing
        let sign_start = Instant::now();
        let signature = crypto::sign_message(&data, &private_key).unwrap();
        let sign_duration = sign_start.elapsed();
        
        // Benchmark verification
        let verify_start = Instant::now();
        let result = crypto::verify_signature(&data, &signature, &public_key).unwrap();
        let verify_duration = verify_start.elapsed();
        
        // Verify that signature verification succeeds
        assert!(result);
        
        // Output performance metrics
        println!(
            "Size: {:.2} KB | Signing: {:.2?} | Verification: {:.2?}",
            size as f64 / 1024.0,
            sign_duration,
            verify_duration,
        );
    }
}

#[test]
fn test_hash_performance() {
    // Test with different file sizes
    let sizes = [
        1024,              // 1KB
        1024 * 10,         // 10KB
        1024 * 100,        // 100KB
        1024 * 1024,       // 1MB
    ];
    
    for &size in &sizes {
        // Create a temporary file with specified size
        let mut temp_file = NamedTempFile::new().unwrap();
        let data = vec![0x42; size];
        temp_file.write_all(&data).unwrap();
        temp_file.flush().unwrap();
        
        // Benchmark hashing
        let start = Instant::now();
        let _hash = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(crypto::compute_file_hash(temp_file.path()))
            .unwrap();
        let duration = start.elapsed();
        
        // Output performance metrics
        println!(
            "Size: {:.2} KB | Hashing: {:.2?} | Throughput: {:.2} MB/s",
            size as f64 / 1024.0,
            duration,
            (size as f64 / duration.as_secs_f64()) / (1024.0 * 1024.0),
        );
    }
} 