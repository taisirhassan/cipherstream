use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// Protocol request types for file transfer operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub enum ProtocolRequest {
    /// Initial handshake request to start a file transfer
    HandshakeRequest {
        filename: String,
        filesize: u64,
        transfer_id: String,
    },
    /// File chunk data
    FileChunk {
        transfer_id: String,
        chunk_index: u64,
        total_chunks: u64,
        data: Vec<u8>,
        is_last: bool,
    },
    /// Cancel an ongoing transfer
    CancelTransfer { transfer_id: String },
}

/// Protocol response types for file transfer operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub enum ProtocolResponse {
    /// Response to handshake request
    HandshakeResponse {
        accepted: bool,
        reason: Option<String>,
        transfer_id: Option<String>,
    },
    /// Response to file chunk
    ChunkResponse {
        transfer_id: String,
        chunk_index: u64,
        success: bool,
        error: Option<String>,
    },
    /// Transfer completion notification
    TransferComplete {
        transfer_id: String,
        success: bool,
        error: Option<String>,
    },
}

/// File metadata used in protocol messages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct FileMetadata {
    pub filename: String,
    pub size: u64,
    pub checksum: String,
    pub encrypted: bool,
}
