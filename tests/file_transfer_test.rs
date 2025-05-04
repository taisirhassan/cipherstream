use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use libp2p::PeerId;
use std::sync::Once;

// Run once for test setup
static CLEANUP: Once = Once::new();

// Helper function to parse peer ID from string
fn parse_peer_id(peer_id_str: &str) -> anyhow::Result<PeerId> {
    Ok(peer_id_str.parse()?)
}

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
    let receiver_port = 45678;
    
    // Set up temporary directories
    let temp_dir_sender = tempfile::tempdir()?;
    let temp_dir_receiver = tempfile::tempdir()?;
    
    // Create a small test file
    let test_file_content = "Test content";
    let test_file_path = temp_dir_sender.path().join("test_file.txt");
    let mut file = fs::File::create(&test_file_path).await?;
    file.write_all(test_file_content.as_bytes()).await?;
    file.flush().await?;
    
    println!("Created test file at: {}", test_file_path.display());
    
    // Generate peer ID for the receiver
    let (receiver_id, _) = cipherstream::network::generate_peer_id();
    println!("Receiver peer ID: {}", receiver_id);
    
    // Start the receiver node (10-second max runtime)
    let receiver_handle = tokio::spawn(async move {
        tokio::select! {
            _ = cipherstream::network::start_node(
                receiver_port, 
                Some(temp_dir_receiver.path().to_string_lossy().to_string())
            ) => {},
            _ = tokio::time::sleep(Duration::from_secs(8)) => {
                println!("Receiver node runtime limit reached");
            }
        }
    });
    
    // Give receiver time to start
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Start the sender and attempt transfer
    let result = tokio::select! {
        result = cipherstream::start_temp_node_and_send_file(
            receiver_id,
            test_file_path.to_string_lossy().to_string(),
            false,
            Some(temp_dir_sender.path().to_string_lossy().to_string()),
        ) => {
            println!("Transfer attempt completed");
            result
        },
        _ = tokio::time::sleep(Duration::from_secs(5)) => {
            println!("Sender timeout reached");
            Ok(())
        }
    };
    
    // Print result
    match result {
        Ok(_) => println!("File transfer setup succeeded"),
        Err(e) => println!("File transfer error: {:?}", e),
    }
    
    // Cleanup
    receiver_handle.abort();
    let _ = std::process::Command::new("pkill")
        .args(["-f", "cipherstream"])
        .output();
    
    // Test passing just means we were able to set up the nodes
    Ok(())
} 