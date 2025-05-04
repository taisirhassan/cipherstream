pub mod crypto;
pub mod discovery;
pub mod file_transfer;
pub mod network;
pub mod protocol;
pub mod utils;

// Re-export key modules for easier access in integration tests
pub use crypto::{encrypt, decrypt, generate_key};

// Re-export for easy access in tests
pub use network::start_node;
pub use network::start_temp_node_and_send_file; 