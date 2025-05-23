use tokio::fs;
use tokio::io::AsyncWriteExt;

// This test verifies we can create nodes and attempt file transfers
// Note: This test is primarily for debugging connection issues
#[tokio::test]
#[ignore] // Marking as ignored by default as it requires actual network I/O
async fn test_file_transfer_setup() -> anyhow::Result<()> {
    // Clean up any previous processes
    let _ = std::process::Command::new("pkill")
        .args(["-f", "cipherstream"])
        .output();
    
    println!("Starting file transfer test (simplified)");
    
    // Use high ports to avoid conflicts
    let _receiver_port = 45678;
    
    // Set up temporary directories
    let temp_dir_sender = tempfile::tempdir()?;
    
    // Create a small test file
    let test_file_content = "Test content";
    let test_file_path = temp_dir_sender.path().join("test_file.txt");
    let mut file = fs::File::create(&test_file_path).await?;
    file.write_all(test_file_content.as_bytes()).await?;
    file.flush().await?;
    
    println!("Created test file at: {}", test_file_path.display());
    
    // Generate peer ID for the receiver using new architecture
    let (receiver_id, _) = cipherstream::infrastructure::NetworkServiceImpl::generate_peer_id();
    println!("Receiver peer ID: {}", receiver_id.as_str());
    
    // TODO: Update this test to use the new modular architecture
    // For now, we'll just verify that we can create the necessary components
    
    // Create application config
    let mut config = cipherstream::AppConfig::default();
    config.default_port = _receiver_port;
    
    // Initialize application service
    let _app_service = cipherstream::ApplicationService::new(config).await
        .map_err(|e| anyhow::anyhow!("Failed to create ApplicationService: {}", e))?;
    
    // Initialize network service
    let _network_service = cipherstream::infrastructure::NetworkServiceImpl::new();
    
    println!("Successfully initialized new modular architecture components");
    
    // Test crypto functionality
    let key = cipherstream::crypto::generate_key()
        .map_err(|e| anyhow::anyhow!("Failed to generate key: {}", e))?;
    let encrypted = cipherstream::crypto::encrypt(test_file_content.as_bytes(), &key)
        .map_err(|e| anyhow::anyhow!("Failed to encrypt: {}", e))?;
    let decrypted = cipherstream::crypto::decrypt(&encrypted, &key)
        .map_err(|e| anyhow::anyhow!("Failed to decrypt: {}", e))?;
    assert_eq!(decrypted, test_file_content.as_bytes());
    
    println!("Crypto functionality verified");
    
    // Cleanup
    let _ = std::process::Command::new("pkill")
        .args(["-f", "cipherstream"])
        .output();
    
    // Test passing means we were able to set up the new architecture components
    println!("File transfer test setup completed successfully with new architecture");
    Ok(())
} 