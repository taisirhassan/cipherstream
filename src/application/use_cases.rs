use crate::core::{domain::*, services::*, traits::*};
use std::sync::Arc;

/// Use case for sending a file to another peer
pub struct SendFileUseCase {
    transfer_service: Arc<TransferDomainService>,
    peer_service: Arc<PeerDomainService>,
}

impl SendFileUseCase {
    pub fn new(
        transfer_service: Arc<TransferDomainService>,
        peer_service: Arc<PeerDomainService>,
    ) -> Self {
        Self {
            transfer_service,
            peer_service,
        }
    }

    /// Execute the send file use case
    pub async fn execute(
        &self,
        file_path: &str,
        sender: PeerId,
        receiver_id: &str,
    ) -> DomainResult<Transfer> {
        // Parse receiver peer ID
        let receiver = PeerId::from_string(receiver_id.to_string());

        // Verify receiver is connected
        let connected_peers = self.peer_service.get_connected_peers().await?;
        if !connected_peers.iter().any(|p| p.id == receiver) {
            return Err("Receiver peer is not connected".into());
        }

        // Initiate the transfer
        self.transfer_service
            .initiate_transfer(file_path, sender, receiver)
            .await
    }
}

/// Use case for listing available files
pub struct ListFilesUseCase {
    file_service: Arc<FileDomainService>,
}

impl ListFilesUseCase {
    pub fn new(file_service: Arc<FileDomainService>) -> Self {
        Self { file_service }
    }

    /// Execute the list files use case
    pub async fn execute(&self) -> DomainResult<Vec<File>> {
        self.file_service.list_files().await
    }
}

/// Use case for listing discovered peers
pub struct ListPeersUseCase {
    peer_service: Arc<PeerDomainService>,
}

impl ListPeersUseCase {
    pub fn new(peer_service: Arc<PeerDomainService>) -> Self {
        Self { peer_service }
    }

    /// Execute the list peers use case
    pub async fn execute(&self) -> DomainResult<Vec<Peer>> {
        self.peer_service.get_connected_peers().await
    }
}

/// Use case for adding a file to the system
pub struct AddFileUseCase {
    file_service: Arc<FileDomainService>,
}

impl AddFileUseCase {
    pub fn new(file_service: Arc<FileDomainService>) -> Self {
        Self { file_service }
    }

    /// Execute the add file use case
    pub async fn execute(&self, file_path: &str) -> DomainResult<File> {
        // Validate file exists and is readable
        if !std::path::Path::new(file_path).exists() {
            return Err("File does not exist".into());
        }

        self.file_service.add_file(file_path).await
    }
}

/// Use case for accepting an incoming transfer
pub struct AcceptTransferUseCase {
    transfer_service: Arc<TransferDomainService>,
}

impl AcceptTransferUseCase {
    pub fn new(transfer_service: Arc<TransferDomainService>) -> Self {
        Self { transfer_service }
    }

    /// Execute the accept transfer use case
    pub async fn execute(&self, transfer_id: &str) -> DomainResult<()> {
        let transfer_id = TransferId::from_string(transfer_id.to_string());
        self.transfer_service.accept_transfer(&transfer_id).await
    }
}

/// Use case for cancelling a transfer
pub struct CancelTransferUseCase {
    transfer_service: Arc<TransferDomainService>,
}

impl CancelTransferUseCase {
    pub fn new(transfer_service: Arc<TransferDomainService>) -> Self {
        Self { transfer_service }
    }

    /// Execute the cancel transfer use case
    pub async fn execute(&self, transfer_id: &str) -> DomainResult<()> {
        let transfer_id = TransferId::from_string(transfer_id.to_string());
        self.transfer_service.cancel_transfer(&transfer_id).await
    }
}

/// Container for all use cases
pub struct UseCases {
    pub send_file: SendFileUseCase,
    pub list_files: ListFilesUseCase,
    pub list_peers: ListPeersUseCase,
    pub add_file: AddFileUseCase,
    pub accept_transfer: AcceptTransferUseCase,
    pub cancel_transfer: CancelTransferUseCase,
}

impl UseCases {
    pub fn new(
        transfer_service: Arc<TransferDomainService>,
        peer_service: Arc<PeerDomainService>,
        file_service: Arc<FileDomainService>,
    ) -> Self {
        Self {
            send_file: SendFileUseCase::new(transfer_service.clone(), peer_service.clone()),
            list_files: ListFilesUseCase::new(file_service.clone()),
            list_peers: ListPeersUseCase::new(peer_service.clone()),
            add_file: AddFileUseCase::new(file_service.clone()),
            accept_transfer: AcceptTransferUseCase::new(transfer_service.clone()),
            cancel_transfer: CancelTransferUseCase::new(transfer_service),
        }
    }
}
