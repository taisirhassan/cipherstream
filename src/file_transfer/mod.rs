pub mod request_handler;
pub mod types;

// Re-exports for easier access from crate::file_transfer::{...}
pub use request_handler::{FileTransferCodec, FileTransferProtocol};
pub use types::{FileMetadata, ProtocolRequest, ProtocolResponse};

// Avoid wildcard re-exports to keep the public API explicit and lints clean
