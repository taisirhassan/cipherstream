use async_trait::async_trait;
use std::error::Error;
use super::domain::*;

/// Result type for domain operations
pub type DomainResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

/// Repository trait for file operations
#[async_trait]
pub trait FileRepository: Send + Sync {
    async fn save_file(&self, file: &File) -> DomainResult<()>;
    async fn find_file_by_id(&self, id: &FileId) -> DomainResult<Option<File>>;
    async fn find_files_by_name(&self, name: &str) -> DomainResult<Vec<File>>;
    async fn list_all_files(&self) -> DomainResult<Vec<File>>;
    async fn delete_file(&self, id: &FileId) -> DomainResult<()>;
}

/// Repository trait for transfer operations
#[async_trait]
pub trait TransferRepository: Send + Sync {
    async fn save_transfer(&self, transfer: &Transfer) -> DomainResult<()>;
    async fn find_transfer_by_id(&self, id: &TransferId) -> DomainResult<Option<Transfer>>;
    async fn find_transfers_by_sender(&self, sender: &PeerId) -> DomainResult<Vec<Transfer>>;
    async fn find_transfers_by_receiver(&self, receiver: &PeerId) -> DomainResult<Vec<Transfer>>;
    async fn list_active_transfers(&self) -> DomainResult<Vec<Transfer>>;
    async fn update_transfer_status(&self, id: &TransferId, status: TransferStatus) -> DomainResult<()>;
    async fn update_transfer_progress(&self, id: &TransferId, progress: TransferProgress) -> DomainResult<()>;
}

/// Repository trait for peer operations
#[async_trait]
pub trait PeerRepository: Send + Sync {
    async fn save_peer(&self, peer: &Peer) -> DomainResult<()>;
    async fn find_peer_by_id(&self, id: &PeerId) -> DomainResult<Option<Peer>>;
    async fn list_connected_peers(&self) -> DomainResult<Vec<Peer>>;
    async fn list_all_peers(&self) -> DomainResult<Vec<Peer>>;
    async fn update_peer_connection_status(&self, id: &PeerId, connected: bool) -> DomainResult<()>;
}

/// Service trait for file operations
#[async_trait]
pub trait FileService: Send + Sync {
    async fn add_file(&self, path: &str) -> DomainResult<File>;
    async fn calculate_file_hash(&self, path: &str) -> DomainResult<String>;
    async fn get_file_metadata(&self, path: &str) -> DomainResult<(String, u64)>; // (name, size)
    async fn read_file_chunk(&self, path: &str, offset: u64, size: usize) -> DomainResult<Vec<u8>>;
    async fn write_file_chunk(&self, path: &str, offset: u64, data: &[u8]) -> DomainResult<()>;
}

/// Service trait for transfer operations
#[async_trait]
pub trait TransferService: Send + Sync {
    async fn initiate_transfer(&self, file_path: &str, receiver: PeerId) -> DomainResult<Transfer>;
    async fn accept_transfer(&self, transfer_id: &TransferId) -> DomainResult<()>;
    async fn reject_transfer(&self, transfer_id: &TransferId, reason: &str) -> DomainResult<()>;
    async fn cancel_transfer(&self, transfer_id: &TransferId) -> DomainResult<()>;
    async fn send_chunk(&self, transfer_id: &TransferId, chunk: Chunk) -> DomainResult<()>;
    async fn receive_chunk(&self, transfer_id: &TransferId, chunk: Chunk) -> DomainResult<()>;
    async fn complete_transfer(&self, transfer_id: &TransferId) -> DomainResult<()>;
}

/// Service trait for peer discovery and management
#[async_trait]
pub trait PeerService: Send + Sync {
    async fn discover_peers(&self) -> DomainResult<Vec<Peer>>;
    async fn connect_to_peer(&self, peer_id: &PeerId) -> DomainResult<()>;
    async fn disconnect_from_peer(&self, peer_id: &PeerId) -> DomainResult<()>;
    async fn get_peer_addresses(&self, peer_id: &PeerId) -> DomainResult<Vec<String>>;
}

/// Service trait for network operations
#[async_trait]
pub trait NetworkService: Send + Sync {
    async fn start_listening(&self, port: u16) -> DomainResult<()>;
    async fn send_message(&self, peer_id: &PeerId, message: Vec<u8>) -> DomainResult<()>;
    async fn broadcast_message(&self, message: Vec<u8>) -> DomainResult<()>;
}

/// Event handler trait for domain events
#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle_event(&self, event: DomainEvent) -> DomainResult<()>;
}

/// Event publisher trait for publishing domain events
#[async_trait]
pub trait EventPublisher: Send + Sync {
    async fn publish(&self, event: DomainEvent) -> DomainResult<()>;
    fn subscribe(&self, handler: Box<dyn EventHandler>) -> DomainResult<()>;
}

/// Configuration trait for accessing application configuration
pub trait Configuration: Send + Sync {
    fn get_data_directory(&self) -> &str;
    fn get_download_directory(&self) -> &str;
    fn get_default_port(&self) -> u16;
    fn get_max_concurrent_transfers(&self) -> usize;
    fn get_chunk_size(&self) -> usize;
} 