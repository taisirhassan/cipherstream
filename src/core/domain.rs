use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

/// Core domain entity representing a peer in the network
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PeerId {
    pub id: String,
}

impl PeerId {
    pub fn new(id: String) -> Self {
        Self { id }
    }
    
    pub fn from_string(id: String) -> Self {
        Self { id }
    }
    
    pub fn as_str(&self) -> &str {
        &self.id
    }
}

impl From<libp2p::PeerId> for PeerId {
    fn from(peer_id: libp2p::PeerId) -> Self {
        Self { id: peer_id.to_string() }
    }
}

impl From<PeerId> for String {
    fn from(peer_id: PeerId) -> String {
        peer_id.id
    }
}

impl Serialize for PeerId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.id.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PeerId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let id = String::deserialize(deserializer)?;
        Ok(PeerId::new(id))
    }
}

/// Domain entity representing a file in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    pub id: FileId,
    pub name: String,
    pub size: u64,
    pub hash: String,
    pub path: String,
    pub created_at: SystemTime,
    pub modified_at: Option<SystemTime>,
}

/// Strongly typed file identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileId(pub String);

impl FileId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
    
    pub fn from_string(id: String) -> Self {
        Self(id)
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for FileId {
    fn default() -> Self { Self::new() }
}

/// Domain entity representing a file transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transfer {
    pub id: TransferId,
    pub file: File,
    pub sender: PeerId,
    pub receiver: PeerId,
    pub status: TransferStatus,
    pub progress: TransferProgress,
    pub started_at: SystemTime,
    pub completed_at: Option<SystemTime>,
}

/// Strongly typed transfer identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransferId(pub String);

impl TransferId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
    
    pub fn from_string(id: String) -> Self {
        Self(id)
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for TransferId {
    fn default() -> Self { Self::new() }
}

/// Transfer status enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransferStatus {
    Pending,
    InProgress,
    Completed,
    Failed { reason: String },
    Cancelled,
}

/// Transfer progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferProgress {
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub chunks_transferred: u64,
    pub total_chunks: u64,
    pub percentage: f32,
}

impl TransferProgress {
    pub fn new(total_bytes: u64, total_chunks: u64) -> Self {
        Self {
            bytes_transferred: 0,
            total_bytes,
            chunks_transferred: 0,
            total_chunks,
            percentage: 0.0,
        }
    }
    
    pub fn update(&mut self, bytes_transferred: u64, chunks_transferred: u64) {
        self.bytes_transferred = bytes_transferred;
        self.chunks_transferred = chunks_transferred;
        self.percentage = if self.total_bytes > 0 {
            (self.bytes_transferred as f32 / self.total_bytes as f32) * 100.0
        } else {
            0.0
        };
    }
    
    pub fn is_complete(&self) -> bool {
        self.bytes_transferred >= self.total_bytes && self.chunks_transferred >= self.total_chunks
    }
}

/// Network peer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
    pub id: PeerId,
    pub addresses: Vec<String>, // Multiaddr as strings for serialization
    pub last_seen: SystemTime,
    pub is_connected: bool,
}

/// Chunk of file data
#[derive(Debug, Clone)]
pub struct Chunk {
    pub index: u64,
    pub data: Vec<u8>,
    pub is_last: bool,
}

/// Domain events that can occur in the system
#[derive(Debug, Clone)]
pub enum DomainEvent {
    PeerDiscovered { peer: Peer },
    PeerConnected { peer_id: PeerId },
    PeerDisconnected { peer_id: PeerId },
    TransferStarted { transfer: Box<Transfer> },
    TransferProgress { transfer_id: TransferId, progress: TransferProgress },
    TransferCompleted { transfer_id: TransferId },
    TransferFailed { transfer_id: TransferId, reason: String },
    ChunkReceived { transfer_id: TransferId, chunk: Chunk },
} 