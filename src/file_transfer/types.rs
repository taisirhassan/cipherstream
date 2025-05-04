use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Information about a file available for transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// Unique identifier for the file
    pub id: String,
    /// Name of the file
    pub name: String,
    /// Size of the file in bytes
    pub size: u64,
    /// SHA-256 hash of the file
    pub hash: String,
    /// Time the file was last modified
    pub modified: Option<SystemTime>,
    /// MIME type of the file if known
    pub mime_type: Option<String>,
}

/// Progress information during transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferProgress {
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub percentage: f32,
    pub transfer_id: String,
    pub file_info: FileInfo,
    pub chunks_transferred: u64,
    pub total_chunks: u64,
    pub start_time: SystemTime,
    pub local_path: String,
}

/// File transfer request messages for the protocol
#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub enum ProtocolRequest {
    /// Initial handshake request to initiate a file transfer
    HandshakeRequest {
        filename: String,
        filesize: u64,
        encrypted: bool,
        transfer_id: String,
    },
    /// File data chunk request
    FileChunk {
        transfer_id: String,
        chunk_index: u64,
        total_chunks: u64,
        data: Vec<u8>,
        is_last: bool,
    },
    /// Request to cancel a transfer
    CancelTransfer {
        transfer_id: String,
    },
}

/// File transfer response messages for the protocol
#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub enum ProtocolResponse {
    /// Response to the handshake
    HandshakeResponse {
        accepted: bool,
        reason: Option<String>,
        transfer_id: Option<String>,
    },
    /// Response to a file chunk
    ChunkResponse {
        transfer_id: String,
        chunk_index: u64,
        success: bool,
        error: Option<String>,
    },
    /// Response to a transfer completion
    TransferComplete {
        transfer_id: String,
        success: bool, 
        error: Option<String>,
    },
}

/// A request to transfer a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileRequest {
    /// Request metadata about available files
    ListFiles,
    /// Request to download a specific file
    DownloadFile {
        /// File identifier
        file_id: String,
        /// Range to download (start, end), if None, downloads the entire file
        range: Option<(u64, u64)>,
    },
    /// Request to upload a file
    UploadFile {
        /// File metadata
        file_info: FileInfo,
    },
    /// Cancel an ongoing transfer
    CancelTransfer {
        /// Transfer identifier
        transfer_id: String,
    },
}

/// Response to a file transfer request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileResponse {
    /// List of available files
    FileList {
        /// Vector of available files
        files: Vec<FileInfo>,
    },
    /// Download accepted
    DownloadAccepted {
        /// Transfer identifier
        transfer_id: String,
        /// File information
        file_info: FileInfo,
        /// Total number of chunks
        total_chunks: u64,
    },
    /// Download rejected
    DownloadRejected {
        /// File identifier
        file_id: String,
        /// Reason for rejection
        reason: String,
    },
    /// File chunk data
    FileChunk {
        /// Transfer identifier
        transfer_id: String,
        /// Chunk index
        chunk_index: u64,
        /// Total chunks
        total_chunks: u64,
        /// Raw chunk data
        data: Vec<u8>,
    },
    /// Upload accepted
    UploadAccepted {
        /// Transfer identifier
        transfer_id: String,
    },
    /// Upload rejected
    UploadRejected {
        /// Reason for rejection
        reason: String,
    },
    /// Transfer complete
    TransferComplete {
        /// Transfer identifier
        transfer_id: String,
    },
    /// Error
    Error {
        /// Error message
        message: String,
    },
} 