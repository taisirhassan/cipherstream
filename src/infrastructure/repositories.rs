use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use async_trait::async_trait;
use crate::core::{domain::*, traits::*};

/// In-memory repository for files (could be replaced with database implementation)
pub struct InMemoryFileRepository {
    files: Arc<RwLock<HashMap<FileId, File>>>,
}

impl InMemoryFileRepository {
    pub fn new() -> Self {
        Self {
            files: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl FileRepository for InMemoryFileRepository {
    async fn save_file(&self, file: &File) -> DomainResult<()> {
        let mut files = self.files.write().unwrap();
        files.insert(file.id.clone(), file.clone());
        Ok(())
    }

    async fn find_file_by_id(&self, id: &FileId) -> DomainResult<Option<File>> {
        let files = self.files.read().unwrap();
        Ok(files.get(id).cloned())
    }

    async fn find_files_by_name(&self, name: &str) -> DomainResult<Vec<File>> {
        let files = self.files.read().unwrap();
        let matching_files: Vec<File> = files
            .values()
            .filter(|file| file.name.contains(name))
            .cloned()
            .collect();
        Ok(matching_files)
    }

    async fn list_all_files(&self) -> DomainResult<Vec<File>> {
        let files = self.files.read().unwrap();
        Ok(files.values().cloned().collect())
    }

    async fn delete_file(&self, id: &FileId) -> DomainResult<()> {
        let mut files = self.files.write().unwrap();
        files.remove(id);
        Ok(())
    }
}

/// In-memory repository for transfers
pub struct InMemoryTransferRepository {
    transfers: Arc<RwLock<HashMap<TransferId, Transfer>>>,
}

impl InMemoryTransferRepository {
    pub fn new() -> Self {
        Self {
            transfers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl TransferRepository for InMemoryTransferRepository {
    async fn save_transfer(&self, transfer: &Transfer) -> DomainResult<()> {
        let mut transfers = self.transfers.write().unwrap();
        transfers.insert(transfer.id.clone(), transfer.clone());
        Ok(())
    }

    async fn find_transfer_by_id(&self, id: &TransferId) -> DomainResult<Option<Transfer>> {
        let transfers = self.transfers.read().unwrap();
        Ok(transfers.get(id).cloned())
    }

    async fn find_transfers_by_sender(&self, sender: &PeerId) -> DomainResult<Vec<Transfer>> {
        let transfers = self.transfers.read().unwrap();
        let matching_transfers: Vec<Transfer> = transfers
            .values()
            .filter(|transfer| transfer.sender == *sender)
            .cloned()
            .collect();
        Ok(matching_transfers)
    }

    async fn find_transfers_by_receiver(&self, receiver: &PeerId) -> DomainResult<Vec<Transfer>> {
        let transfers = self.transfers.read().unwrap();
        let matching_transfers: Vec<Transfer> = transfers
            .values()
            .filter(|transfer| transfer.receiver == *receiver)
            .cloned()
            .collect();
        Ok(matching_transfers)
    }

    async fn list_active_transfers(&self) -> DomainResult<Vec<Transfer>> {
        let transfers = self.transfers.read().unwrap();
        let active_transfers: Vec<Transfer> = transfers
            .values()
            .filter(|transfer| matches!(transfer.status, TransferStatus::InProgress | TransferStatus::Pending))
            .cloned()
            .collect();
        Ok(active_transfers)
    }

    async fn update_transfer_status(&self, id: &TransferId, status: TransferStatus) -> DomainResult<()> {
        let mut transfers = self.transfers.write().unwrap();
        if let Some(transfer) = transfers.get_mut(id) {
            transfer.status = status;
        }
        Ok(())
    }

    async fn update_transfer_progress(&self, id: &TransferId, progress: TransferProgress) -> DomainResult<()> {
        let mut transfers = self.transfers.write().unwrap();
        if let Some(transfer) = transfers.get_mut(id) {
            transfer.progress = progress;
        }
        Ok(())
    }
}

/// In-memory repository for peers
pub struct InMemoryPeerRepository {
    peers: Arc<RwLock<HashMap<PeerId, Peer>>>,
}

impl InMemoryPeerRepository {
    pub fn new() -> Self {
        Self {
            peers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl PeerRepository for InMemoryPeerRepository {
    async fn save_peer(&self, peer: &Peer) -> DomainResult<()> {
        let mut peers = self.peers.write().unwrap();
        peers.insert(peer.id.clone(), peer.clone());
        Ok(())
    }

    async fn find_peer_by_id(&self, id: &PeerId) -> DomainResult<Option<Peer>> {
        let peers = self.peers.read().unwrap();
        Ok(peers.get(id).cloned())
    }

    async fn list_connected_peers(&self) -> DomainResult<Vec<Peer>> {
        let peers = self.peers.read().unwrap();
        let connected_peers: Vec<Peer> = peers
            .values()
            .filter(|peer| peer.is_connected)
            .cloned()
            .collect();
        Ok(connected_peers)
    }

    async fn list_all_peers(&self) -> DomainResult<Vec<Peer>> {
        let peers = self.peers.read().unwrap();
        Ok(peers.values().cloned().collect())
    }

    async fn update_peer_connection_status(&self, id: &PeerId, connected: bool) -> DomainResult<()> {
        let mut peers = self.peers.write().unwrap();
        if let Some(peer) = peers.get_mut(id) {
            peer.is_connected = connected;
        }
        Ok(())
    }
}

/// Builder for creating repository instances
pub struct RepositoryBuilder;

impl RepositoryBuilder {
    pub fn build_file_repository() -> Arc<dyn FileRepository> {
        Arc::new(InMemoryFileRepository::new())
    }

    pub fn build_transfer_repository() -> Arc<dyn TransferRepository> {
        Arc::new(InMemoryTransferRepository::new())
    }

    pub fn build_peer_repository() -> Arc<dyn PeerRepository> {
        Arc::new(InMemoryPeerRepository::new())
    }
} 