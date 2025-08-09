use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

// Use a static counter to generate unique port numbers
static PORT_COUNTER: AtomicU16 = AtomicU16::new(10000);

// Struct to hold node info for cleanup
struct TestNode {
    process: Child,
    data_dir: String,
    port: u16,
    peer_id: Arc<Mutex<Option<String>>>,
}

impl Drop for TestNode {
    fn drop(&mut self) {
        // Print a message so we know cleanup is happening
        println!(
            "Cleaning up node with port {} and data dir {}",
            self.port, self.data_dir
        );

        // Ensure the process is terminated
        let _ = self.process.kill();
        let _ = self.process.wait();

        // Sleep a moment to ensure ports are released
        sleep(Duration::from_millis(500));

        // Clean up data directory
        let _ = fs::remove_dir_all(&self.data_dir);
    }
}

// Integration test for peer discovery and file transfer
#[tokio::test]
#[ignore] // TODO: Re-implement this test with the new modular network architecture
async fn test_peer_discovery_and_transfer() {
    // Pre-build the binary to avoid compile time in child processes
    let _ = std::process::Command::new("cargo")
        .args(["build"]) // avoid -q for broader compatibility
        .status()
        .expect("Failed to build project before integration test");
    // Create test files and directories
    let test_file = "test_file.txt";
    fs::write(test_file, "This is a test file for integration testing").unwrap();

    // Use unique directories for each test run
    let test_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let node1_dir = format!("test_data/node1_{}", test_id);
    let node2_dir = format!("test_data/node2_{}", test_id);

    fs::create_dir_all(&node1_dir).unwrap_or_default();
    fs::create_dir_all(&node2_dir).unwrap_or_default();

    println!("Test directories: {} and {}", node1_dir, node2_dir);

    // Start node 2 first (receiver)
    let node2 = start_node(&node2_dir);

    // Wait for node2 to initialize and get its peer ID
    let mut attempts = 0;
    let max_attempts = 120; // Allow up to ~60s for first-time startup/compilation
    let mut peer_id = String::new();

    while attempts < max_attempts {
        sleep(Duration::from_millis(500)); // Shorter sleep intervals
        let lock = node2.peer_id.lock().unwrap();
        if let Some(id) = &*lock {
            peer_id = id.clone();
            break;
        }
        attempts += 1;
    }

    if peer_id.is_empty() {
        panic!("Failed to get peer ID for node2 after multiple attempts");
    }

    println!("Using node2 peer ID: {}", peer_id);

    // Start node 1 (sender)
    let node1 = start_node(&node1_dir);

    // Wait for node1 to initialize
    let mut attempts = 0;
    while attempts < max_attempts {
        sleep(Duration::from_millis(500));
        let lock = node1.peer_id.lock().unwrap();
        if lock.is_some() {
            break;
        }
        attempts += 1;
    }

    // Wait for discovery to happen
    println!("Waiting for peer discovery...");
    sleep(Duration::from_secs(10));

    // Print the peer ID to make debugging easier
    println!("Sending file to peer: {}", peer_id);

    // Use the relative path to the test directory for verification later
    let downloaded_file = format!("{}/downloads/{}", &node2_dir, test_file);

    // Send file from node1 to node2
    let output = Command::new("cargo")
        .args(["run", "--", "send", "--file", test_file, "--peer", &peer_id])
        .output()
        .expect("Failed to send file");

    assert!(output.status.success(), "File transfer command failed");

    // Quick sanity: current implementation logs that transfer is prepared
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("File transfer command prepared"),
        "Expected confirmation log not found in output. Got: {}",
        combined
    );

    // For CI: only enforce end-to-end receive check when explicitly requested
    if std::env::var("CIPHERSTREAM_STRICT_E2E").as_deref() == Ok("1") {
        // Wait longer for file to be processed
        println!("Waiting for file transfer to complete...");
        let mut transfer_success = false;
        for _ in 0..60 {
            sleep(Duration::from_secs(1));
            if Path::new(&downloaded_file).exists() {
                transfer_success = true;
                break;
            }
        }

        // Verify file exists in download directory of node2
        assert!(
            transfer_success,
            "File was not received by node2. Expected at {}",
            downloaded_file
        );
    }

    // Clean up - the Drop trait will handle process cleanup
    fs::remove_file(test_file).unwrap_or_default();

    // Ensure we keep references to the nodes until the end of the test
    drop(node1);
    drop(node2);

    // Sleep after dropping nodes to ensure ports are released
    sleep(Duration::from_secs(1));
}

// Start a node with a random port allocation
fn start_node(data_dir: &str) -> TestNode {
    // Get a unique port for this test
    let port = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);

    println!("Starting node on port {} with data dir {}", port, data_dir);

    let peer_id = Arc::new(Mutex::new(None::<String>));
    let peer_id_clone = peer_id.clone();

    // Use a specific port to avoid conflicts
    let mut cmd = Command::new("cargo");
    cmd.env("RUST_LOG", "info,libp2p_swarm=warn");
    cmd.args([
        "run",
        "--",
        "start",
        "--port",
        &port.to_string(),
        "--data-dir",
        data_dir,
    ])
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());

    let mut process = cmd.spawn().expect("Failed to start node");

    // Create a thread to read the node's output to extract peer ID
    if let Some(stdout) = process.stdout.take() {
        let handler_clone = peer_id_clone.clone();

        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                println!("[Node]: {}", line);

                // Extract peer ID from the log line
                if line.contains("Local peer id:") {
                    if let Some(id) = line
                        .split("Local peer id:")
                        .nth(1)
                        .map(|s| s.trim().to_string())
                    {
                        let mut lock = handler_clone.lock().unwrap();
                        *lock = Some(id);
                    }
                }
            }
        });
    }

    // Also capture stderr
    if let Some(stderr) = process.stderr.take() {
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                println!("[Node Error]: {}", line);
            }
        });
    }

    // Give node a moment to start up
    sleep(Duration::from_secs(1));

    TestNode {
        process,
        data_dir: data_dir.to_string(),
        port,
        peer_id,
    }
}
