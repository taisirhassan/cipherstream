use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
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

impl Default for InMemoryFileRepository {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl FileRepository for InMemoryFileRepository {
    async fn save_file(&self, file: &File) -> DomainResult<()> {
        let mut files = self.files.write().await;
        files.insert(file.id.clone(), file.clone());
        Ok(())
    }

    async fn find_file_by_id(&self, id: &FileId) -> DomainResult<Option<File>> {
        let files = self.files.read().await;
        Ok(files.get(id).cloned())
    }

    async fn find_files_by_name(&self, name: &str) -> DomainResult<Vec<File>> {
        let files = self.files.read().await;
        let matching_files: Vec<File> = files
            .values()
            .filter(|file| file.name.contains(name))
            .cloned()
            .collect();
        Ok(matching_files)
    }

    async fn list_all_files(&self) -> DomainResult<Vec<File>> {
        let files = self.files.read().await;
        Ok(files.values().cloned().collect())
    }

    async fn delete_file(&self, id: &FileId) -> DomainResult<()> {
        let mut files = self.files.write().await;
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

impl Default for InMemoryTransferRepository {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl TransferRepository for InMemoryTransferRepository {
    async fn save_transfer(&self, transfer: &Transfer) -> DomainResult<()> {
        let mut transfers = self.transfers.write().await;
        transfers.insert(transfer.id.clone(), transfer.clone());
        Ok(())
    }

    async fn find_transfer_by_id(&self, id: &TransferId) -> DomainResult<Option<Transfer>> {
        let transfers = self.transfers.read().await;
        Ok(transfers.get(id).cloned())
    }

    async fn find_transfers_by_sender(&self, sender: &PeerId) -> DomainResult<Vec<Transfer>> {
        let transfers = self.transfers.read().await;
        let matching_transfers: Vec<Transfer> = transfers
            .values()
            .filter(|transfer| transfer.sender == *sender)
            .cloned()
            .collect();
        Ok(matching_transfers)
    }

    async fn find_transfers_by_receiver(&self, receiver: &PeerId) -> DomainResult<Vec<Transfer>> {
        let transfers = self.transfers.read().await;
        let matching_transfers: Vec<Transfer> = transfers
            .values()
            .filter(|transfer| transfer.receiver == *receiver)
            .cloned()
            .collect();
        Ok(matching_transfers)
    }

    async fn list_active_transfers(&self) -> DomainResult<Vec<Transfer>> {
        let transfers = self.transfers.read().await;
        let active_transfers: Vec<Transfer> = transfers
            .values()
            .filter(|transfer| matches!(transfer.status, TransferStatus::InProgress | TransferStatus::Pending))
            .cloned()
            .collect();
        Ok(active_transfers)
    }

    async fn update_transfer_status(&self, id: &TransferId, status: TransferStatus) -> DomainResult<()> {
        let mut transfers = self.transfers.write().await;
        if let Some(transfer) = transfers.get_mut(id) {
            transfer.status = status;
        }
        Ok(())
    }

    async fn update_transfer_progress(&self, id: &TransferId, progress: TransferProgress) -> DomainResult<()> {
        let mut transfers = self.transfers.write().await;
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

impl Default for InMemoryPeerRepository {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl PeerRepository for InMemoryPeerRepository {
    async fn save_peer(&self, peer: &Peer) -> DomainResult<()> {
        let mut peers = self.peers.write().await;
        peers.insert(peer.id.clone(), peer.clone());
        Ok(())
    }

    async fn find_peer_by_id(&self, id: &PeerId) -> DomainResult<Option<Peer>> {
        let peers = self.peers.read().await;
        Ok(peers.get(id).cloned())
    }

    async fn list_connected_peers(&self) -> DomainResult<Vec<Peer>> {
        let peers = self.peers.read().await;
        let connected_peers: Vec<Peer> = peers
            .values()
            .filter(|peer| peer.is_connected)
            .cloned()
            .collect();
        Ok(connected_peers)
    }

    async fn list_all_peers(&self) -> DomainResult<Vec<Peer>> {
        let peers = self.peers.read().await;
        Ok(peers.values().cloned().collect())
    }

    async fn update_peer_connection_status(&self, id: &PeerId, connected: bool) -> DomainResult<()> {
        let mut peers = self.peers.write().await;
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
        match std::env::var("CIPHERSTREAM_REPO_BACKEND").ok().as_deref() {
            Some("sled") => {
                if let Ok(repo) = SledFileRepository::new() { Arc::new(repo) } else { Arc::new(InMemoryFileRepository::new()) }
            }
            _ => Arc::new(InMemoryFileRepository::new()),
        }
    }

    pub fn build_transfer_repository() -> Arc<dyn TransferRepository> {
        match std::env::var("CIPHERSTREAM_REPO_BACKEND").ok().as_deref() {
            Some("sled") => {
                if let Ok(repo) = SledTransferRepository::new() { Arc::new(repo) } else { Arc::new(InMemoryTransferRepository::new()) }
            }
            _ => Arc::new(InMemoryTransferRepository::new()),
        }
    }

    pub fn build_peer_repository() -> Arc<dyn PeerRepository> {
        match std::env::var("CIPHERSTREAM_REPO_BACKEND").ok().as_deref() {
            Some("sled") => {
                if let Ok(repo) = SledPeerRepository::new() { Arc::new(repo) } else { Arc::new(InMemoryPeerRepository::new()) }
            }
            _ => Arc::new(InMemoryPeerRepository::new()),
        }
    }
} 

// Durable repositories backed by sled
pub struct SledStores {
    _db: sled::Db,
    files: sled::Tree,
    transfers: sled::Tree,
    peers: sled::Tree,
}

impl SledStores {
    fn open() -> Result<Self, Box<dyn std::error::Error>> {
        let path = std::env::var("CIPHERSTREAM_DB_PATH").unwrap_or_else(|_| ".cipherstream_db".to_string());
        let db = sled::open(path)?;
        let files = db.open_tree("files")?;
        let transfers = db.open_tree("transfers")?;
        let peers = db.open_tree("peers")?;
        Ok(Self { _db: db, files, transfers, peers })
    }
}

pub struct SledFileRepository {
    store: SledStores,
}

impl SledFileRepository {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> { Ok(Self { store: SledStores::open()? }) }
}

#[async_trait]
impl FileRepository for SledFileRepository {
    async fn save_file(&self, file: &File) -> DomainResult<()> {
        let key = file.id.as_str().as_bytes().to_vec();
        let value = serde_json::to_vec(file)?;
        let files = self.store.files.clone();
        tokio::task::spawn_blocking(move || files.insert(key, value)).await??;
        Ok(())
    }

    async fn find_file_by_id(&self, id: &FileId) -> DomainResult<Option<File>> {
        let key = id.as_str().as_bytes().to_vec();
        let files = self.store.files.clone();
        let res = tokio::task::spawn_blocking(move || files.get(key)).await??;
        Ok(res.and_then(|ivec| serde_json::from_slice(&ivec).ok()))
    }

    async fn find_files_by_name(&self, name: &str) -> DomainResult<Vec<File>> {
        let name = name.to_string();
        let files = self.store.files.clone();
        let entries: Vec<File> = tokio::task::spawn_blocking(move || {
            files.iter().values().filter_map(|res| res.ok()).filter_map(|v| serde_json::from_slice::<File>(&v).ok()).filter(|f| f.name.contains(&name)).collect()
        }).await?;
        Ok(entries)
    }

    async fn list_all_files(&self) -> DomainResult<Vec<File>> {
        let files = self.store.files.clone();
        let entries: Vec<File> = tokio::task::spawn_blocking(move || {
            files.iter().values().filter_map(|res| res.ok()).filter_map(|v| serde_json::from_slice::<File>(&v).ok()).collect()
        }).await?;
        Ok(entries)
    }

    async fn delete_file(&self, id: &FileId) -> DomainResult<()> {
        let key = id.as_str().as_bytes().to_vec();
        let files = self.store.files.clone();
        tokio::task::spawn_blocking(move || files.remove(key)).await??;
        Ok(())
    }
}

pub struct SledTransferRepository {
    store: SledStores,
}

impl SledTransferRepository {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> { Ok(Self { store: SledStores::open()? }) }
}

#[async_trait]
impl TransferRepository for SledTransferRepository {
    async fn save_transfer(&self, transfer: &Transfer) -> DomainResult<()> {
        let key = transfer.id.as_str().as_bytes().to_vec();
        let value = serde_json::to_vec(transfer)?;
        let t = self.store.transfers.clone();
        tokio::task::spawn_blocking(move || t.insert(key, value)).await??;
        Ok(())
    }

    async fn find_transfer_by_id(&self, id: &TransferId) -> DomainResult<Option<Transfer>> {
        let key = id.as_str().as_bytes().to_vec();
        let t = self.store.transfers.clone();
        let res = tokio::task::spawn_blocking(move || t.get(key)).await??;
        Ok(res.and_then(|ivec| serde_json::from_slice(&ivec).ok()))
    }

    async fn find_transfers_by_sender(&self, sender: &PeerId) -> DomainResult<Vec<Transfer>> {
        let sender_id = sender.as_str().to_string();
        let t = self.store.transfers.clone();
        let entries: Vec<Transfer> = tokio::task::spawn_blocking(move || {
            t.iter().values().filter_map(|res| res.ok()).filter_map(|v| serde_json::from_slice::<Transfer>(&v).ok()).filter(|tr| tr.sender.as_str() == sender_id).collect()
        }).await?;
        Ok(entries)
    }

    async fn find_transfers_by_receiver(&self, receiver: &PeerId) -> DomainResult<Vec<Transfer>> {
        let receiver_id = receiver.as_str().to_string();
        let t = self.store.transfers.clone();
        let entries: Vec<Transfer> = tokio::task::spawn_blocking(move || {
            t.iter().values().filter_map(|res| res.ok()).filter_map(|v| serde_json::from_slice::<Transfer>(&v).ok()).filter(|tr| tr.receiver.as_str() == receiver_id).collect()
        }).await?;
        Ok(entries)
    }

    async fn list_active_transfers(&self) -> DomainResult<Vec<Transfer>> {
        let t = self.store.transfers.clone();
        let entries: Vec<Transfer> = tokio::task::spawn_blocking(move || {
            t.iter().values().filter_map(|res| res.ok()).filter_map(|v| serde_json::from_slice::<Transfer>(&v).ok()).filter(|tr| matches!(tr.status, TransferStatus::InProgress | TransferStatus::Pending)).collect()
        }).await?;
        Ok(entries)
    }

    async fn update_transfer_status(&self, id: &TransferId, status: TransferStatus) -> DomainResult<()> {
        if let Some(mut tr) = self.find_transfer_by_id(id).await? {
            tr.status = status;
            self.save_transfer(&tr).await?
        }
        Ok(())
    }

    async fn update_transfer_progress(&self, id: &TransferId, progress: TransferProgress) -> DomainResult<()> {
        if let Some(mut tr) = self.find_transfer_by_id(id).await? {
            tr.progress = progress;
            self.save_transfer(&tr).await?
        }
        Ok(())
    }
}

pub struct SledPeerRepository {
    store: SledStores,
}

impl SledPeerRepository {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> { Ok(Self { store: SledStores::open()? }) }
}

#[async_trait]
impl PeerRepository for SledPeerRepository {
    async fn save_peer(&self, peer: &Peer) -> DomainResult<()> {
        let key = peer.id.as_str().as_bytes().to_vec();
        let value = serde_json::to_vec(peer)?;
        let p = self.store.peers.clone();
        tokio::task::spawn_blocking(move || p.insert(key, value)).await??;
        Ok(())
    }

    async fn find_peer_by_id(&self, id: &PeerId) -> DomainResult<Option<Peer>> {
        let key = id.as_str().as_bytes().to_vec();
        let p = self.store.peers.clone();
        let res = tokio::task::spawn_blocking(move || p.get(key)).await??;
        Ok(res.and_then(|ivec| serde_json::from_slice(&ivec).ok()))
    }

    async fn list_connected_peers(&self) -> DomainResult<Vec<Peer>> {
        let p = self.store.peers.clone();
        let entries: Vec<Peer> = tokio::task::spawn_blocking(move || {
            p.iter().values().filter_map(|res| res.ok()).filter_map(|v| serde_json::from_slice::<Peer>(&v).ok()).filter(|peer| peer.is_connected).collect()
        }).await?;
        Ok(entries)
    }

    async fn list_all_peers(&self) -> DomainResult<Vec<Peer>> {
        let p = self.store.peers.clone();
        let entries: Vec<Peer> = tokio::task::spawn_blocking(move || {
            p.iter().values().filter_map(|res| res.ok()).filter_map(|v| serde_json::from_slice::<Peer>(&v).ok()).collect()
        }).await?;
        Ok(entries)
    }

    async fn update_peer_connection_status(&self, id: &PeerId, connected: bool) -> DomainResult<()> {
        if let Some(mut peer) = self.find_peer_by_id(id).await? {
            peer.is_connected = connected;
            self.save_peer(&peer).await?
        }
        Ok(())
    }
}