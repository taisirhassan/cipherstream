use serde::{Deserialize, Serialize};

/// DTO for file information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDto {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub hash: String,
    pub path: String,
}

/// DTO for peer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerDto {
    pub id: String,
    pub addresses: Vec<String>,
    pub is_connected: bool,
}

/// DTO for transfer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferDto {
    pub id: String,
    pub file_name: String,
    pub file_size: u64,
    pub sender_id: String,
    pub receiver_id: String,
    pub status: String,
    pub progress_percentage: f32,
    pub bytes_transferred: u64,
}

/// DTO for sending file request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendFileRequest {
    pub file_path: String,
    pub receiver_id: String,
}

/// DTO for sending file response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendFileResponse {
    pub transfer_id: String,
    pub message: String,
}

/// DTO for error responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub details: Option<String>,
} 