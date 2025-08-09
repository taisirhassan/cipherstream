pub mod types;
pub mod request_handler;

// Re-exports for easier access from crate::file_transfer::{...}
pub use request_handler::{FileTransferCodec, FileTransferProtocol};
pub use types::{ProtocolRequest, ProtocolResponse, FileMetadata};

// Avoid wildcard re-exports to keep the public API explicit and lints clean