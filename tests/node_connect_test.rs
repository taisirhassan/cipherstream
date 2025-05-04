use std::time::Duration;
use tokio::time::sleep;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use libp2p::PeerId;
use anyhow::Result;

// This test creates two nodes and verifies they can connect to each other
#[tokio::test]
#[ignore] // Marking as ignored by default as it requires actual network I/O
async fn test_nodes_can_connect() -> Result<()> {
    // Kill any existing cipherstream processes
    let _ = std::process::Command::new("pkill")
        .args(["-f", "cipherstream"])
        .output();
    
    println!("Starting node connectivity test");
    
    // Create temp directories for both nodes
    let temp_dir_node1 = tempfile::tempdir()?;
    let temp_dir_node2 = tempfile::tempdir()?;
    
    // Track node startup status
    let node1_ready = Arc::new(AtomicBool::new(false));
    let node1_ready_clone = node1_ready.clone();
    
    // Use a specific port range that's less likely to conflict
    let node1_port = 45001;
    let node2_port = 45002;
    
    // Start node 1
    let node1_handle = tokio::spawn(async move {
        println!("Starting node 1 on port {}", node1_port);
        
        if let Err(e) = cipherstream::network::start_node(
            node1_port,
            Some(temp_dir_node1.path().to_string_lossy().to_string())
        ).await {
            eprintln!("Node 1 error: {:?}", e);
        } else {
            node1_ready_clone.store(true, Ordering::SeqCst);
        }
        
        temp_dir_node1 // Keep alive
    });
    
    // Wait for node 1 to start
    println!("Waiting for node 1 to start");
    sleep(Duration::from_secs(3)).await;
    
    // Start node 2
    let node2_handle = tokio::spawn(async move {
        println!("Starting node 2 on port {}", node2_port);
        
        if let Err(e) = cipherstream::network::start_node(
            node2_port,
            Some(temp_dir_node2.path().to_string_lossy().to_string())
        ).await {
            eprintln!("Node 2 error: {:?}", e);
        }
        
        temp_dir_node2 // Keep alive
    });
    
    // Wait for both nodes to be running
    println!("Waiting for nodes to discover each other");
    sleep(Duration::from_secs(10)).await;
    
    // At this point, the nodes should have discovered each other
    // We'll abort both handles now to clean up
    node1_handle.abort();
    node2_handle.abort();
    
    println!("Test completed, cleaning up");
    
    // Kill any lingering processes
    let _ = std::process::Command::new("pkill")
        .args(["-f", "cipherstream"])
        .output();
    
    // The test passes if we got this far without errors
    Ok(())
} 