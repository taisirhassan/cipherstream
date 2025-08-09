use super::domain::*;
use super::traits::*;
use std::sync::Arc;
use std::time::SystemTime;

/// Domain service for managing file transfers
pub struct TransferDomainService {
    file_repo: Arc<dyn FileRepository>,
    transfer_repo: Arc<dyn TransferRepository>,
    peer_repo: Arc<dyn PeerRepository>,
    file_service: Arc<dyn FileService>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl TransferDomainService {
    pub fn new(
        file_repo: Arc<dyn FileRepository>,
        transfer_repo: Arc<dyn TransferRepository>,
        peer_repo: Arc<dyn PeerRepository>,
        file_service: Arc<dyn FileService>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            file_repo,
            transfer_repo,
            peer_repo,
            file_service,
            event_publisher,
        }
    }

    /// Initiate a new file transfer
    pub async fn initiate_transfer(
        &self,
        file_path: &str,
        sender: PeerId,
        receiver: PeerId,
    ) -> DomainResult<Transfer> {
        // Verify receiver exists and is connected
        let peer = self
            .peer_repo
            .find_peer_by_id(&receiver)
            .await?
            .ok_or("Receiver peer not found")?;

        if !peer.is_connected {
            return Err("Receiver peer is not connected".into());
        }

        // Get file metadata
        let (file_name, file_size) = self.file_service.get_file_metadata(file_path).await?;
        let file_hash = self.file_service.calculate_file_hash(file_path).await?;

        // Create file entity
        let file = File {
            id: FileId::new(),
            name: file_name,
            size: file_size,
            hash: file_hash,
            path: file_path.to_string(),
            created_at: SystemTime::now(),
            modified_at: None,
        };

        // Calculate chunks
        const CHUNK_SIZE: u64 = 1024 * 1024; // 1MB
        let total_chunks: u64 = file_size.div_ceil(CHUNK_SIZE);

        // Create transfer entity
        let transfer: Transfer = Transfer {
            id: TransferId::new(),
            file: file.clone(),
            sender,
            receiver,
            status: TransferStatus::Pending,
            progress: TransferProgress::new(file_size, total_chunks),
            started_at: SystemTime::now(),
            completed_at: None,
        };

        // Save entities
        self.file_repo.save_file(&file).await?;
        self.transfer_repo.save_transfer(&transfer).await?;

        // Publish event
        self.event_publisher
            .publish(DomainEvent::TransferStarted {
                transfer: Box::new(transfer.clone()),
            })
            .await?;

        Ok(transfer)
    }

    /// Accept an incoming transfer
    pub async fn accept_transfer(&self, transfer_id: &TransferId) -> DomainResult<()> {
        let mut transfer = self
            .transfer_repo
            .find_transfer_by_id(transfer_id)
            .await?
            .ok_or("Transfer not found")?;

        match transfer.status {
            TransferStatus::Pending => {
                transfer.status = TransferStatus::InProgress;
                self.transfer_repo.save_transfer(&transfer).await?;
                Ok(())
            }
            _ => Err("Transfer is not in pending state".into()),
        }
    }

    /// Update transfer progress
    pub async fn update_progress(
        &self,
        transfer_id: &TransferId,
        bytes_transferred: u64,
        chunks_transferred: u64,
    ) -> DomainResult<()> {
        let mut transfer = self
            .transfer_repo
            .find_transfer_by_id(transfer_id)
            .await?
            .ok_or("Transfer not found")?;

        transfer
            .progress
            .update(bytes_transferred, chunks_transferred);

        if transfer.progress.is_complete() {
            transfer.status = TransferStatus::Completed;
            transfer.completed_at = Some(SystemTime::now());

            self.event_publisher
                .publish(DomainEvent::TransferCompleted {
                    transfer_id: transfer_id.clone(),
                })
                .await?;
        } else {
            self.event_publisher
                .publish(DomainEvent::TransferProgress {
                    transfer_id: transfer_id.clone(),
                    progress: transfer.progress.clone(),
                })
                .await?;
        }

        self.transfer_repo.save_transfer(&transfer).await?;
        Ok(())
    }

    /// Cancel a transfer
    pub async fn cancel_transfer(&self, transfer_id: &TransferId) -> DomainResult<()> {
        let mut transfer = self
            .transfer_repo
            .find_transfer_by_id(transfer_id)
            .await?
            .ok_or("Transfer not found")?;

        match transfer.status {
            TransferStatus::Completed => Err("Cannot cancel completed transfer".into()),
            _ => {
                transfer.status = TransferStatus::Cancelled;
                self.transfer_repo.save_transfer(&transfer).await?;
                Ok(())
            }
        }
    }
}

/// Domain service for managing peers
pub struct PeerDomainService {
    peer_repo: Arc<dyn PeerRepository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl PeerDomainService {
    pub fn new(
        peer_repo: Arc<dyn PeerRepository>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            peer_repo,
            event_publisher,
        }
    }

    /// Register a discovered peer
    pub async fn register_discovered_peer(
        &self,
        peer_id: PeerId,
        addresses: Vec<String>,
    ) -> DomainResult<()> {
        let peer = Peer {
            id: peer_id.clone(),
            addresses,
            last_seen: SystemTime::now(),
            is_connected: false,
        };

        self.peer_repo.save_peer(&peer).await?;

        self.event_publisher
            .publish(DomainEvent::PeerDiscovered { peer: peer.clone() })
            .await?;

        Ok(())
    }

    /// Update peer connection status
    pub async fn update_connection_status(
        &self,
        peer_id: &PeerId,
        connected: bool,
    ) -> DomainResult<()> {
        self.peer_repo
            .update_peer_connection_status(peer_id, connected)
            .await?;

        let event = if connected {
            DomainEvent::PeerConnected {
                peer_id: peer_id.clone(),
            }
        } else {
            DomainEvent::PeerDisconnected {
                peer_id: peer_id.clone(),
            }
        };

        self.event_publisher.publish(event).await?;
        Ok(())
    }

    /// Get connected peers
    pub async fn get_connected_peers(&self) -> DomainResult<Vec<Peer>> {
        self.peer_repo.list_connected_peers().await
    }
}

/// Domain service for file operations
pub struct FileDomainService {
    file_repo: Arc<dyn FileRepository>,
    file_service: Arc<dyn FileService>,
}

impl FileDomainService {
    pub fn new(file_repo: Arc<dyn FileRepository>, file_service: Arc<dyn FileService>) -> Self {
        Self {
            file_repo,
            file_service,
        }
    }

    /// Add a file to the system
    pub async fn add_file(&self, file_path: &str) -> DomainResult<File> {
        let file = self.file_service.add_file(file_path).await?;
        self.file_repo.save_file(&file).await?;
        Ok(file)
    }

    /// Get all available files
    pub async fn list_files(&self) -> DomainResult<Vec<File>> {
        self.file_repo.list_all_files().await
    }

    /// Find file by ID
    pub async fn find_file(&self, file_id: &FileId) -> DomainResult<Option<File>> {
        self.file_repo.find_file_by_id(file_id).await
    }
}
