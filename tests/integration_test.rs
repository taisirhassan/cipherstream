use std::path::Path;
use std::process::{Command, Child};
use std::thread::sleep;
use std::time::Duration;
use std::fs;

// Integration test for peer discovery and file transfer
#[tokio::test]
async fn test_peer_discovery_and_transfer() {
    // Create test files
    let test_file = "test_file.txt";
    fs::write(test_file, "This is a test file for integration testing").unwrap();
    
    // Start two nodes
    let mut node1 = start_node(8100);
    let mut node2 = start_node(8101);
    
    // Wait for discovery
    sleep(Duration::from_secs(5));
    
    // Get peer ID of node2 
    let peer_id = get_peer_id(8101);
    
    // Send file from node1 to node2
    let status = Command::new("cargo")
        .args(["run", "--", "send", "--file", test_file, "--peer", &peer_id])
        .status()
        .expect("Failed to send file");
        
    assert!(status.success(), "File transfer failed");
    
    // Verify file exists in download directory of node2
    let download_path = Path::new("downloads").join(test_file);
    sleep(Duration::from_secs(2)); // Wait for file to be processed
    assert!(download_path.exists(), "File was not received by node2");
    
    // Clean up
    node1.kill().expect("Failed to kill node1");
    node2.kill().expect("Failed to kill node2");
    fs::remove_file(test_file).unwrap_or_default();
}

fn start_node(port: u16) -> Child {
    Command::new("cargo")
        .args(["run", "--", "start", "--port", &port.to_string()])
        .spawn()
        .expect("Failed to start node")
}

fn get_peer_id(port: u16) -> String {
    // This would require the ability to query the node's peer ID
    // For now this is a placeholder
    "12D3KooWSomeTestPeerID".to_string()
} 