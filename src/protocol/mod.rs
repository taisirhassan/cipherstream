use std::path::Path;
use libp2p::PeerId;
use tokio::fs::File;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Serialize, Deserialize};
use std::error::Error;
use libp2p::swarm::Swarm;
use uuid::Uuid;
use crate::discovery::Behavior;
use crate::file_transfer::{request_handler, ProtocolRequest, ProtocolResponse};
use libp2p::request_response::OutboundRequestId;

// Will be used when implementing chunks in the future
#[allow(dead_code)]
const CHUNK_SIZE: usize = 1024 * 64; // 64KB chunks
const PROTOCOL_VERSION: &str = "cipherstream/file/1.0.0";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileMetadata {
    pub filename: String,
    pub size: u64,
    pub checksum: String,
    pub encrypted: bool,
}

/// Our custom file transfer protocol
#[derive(Debug, Clone)]
pub struct FileTransferProtocol;

impl FileTransferProtocol {
    pub fn protocol_name(&self) -> &[u8] {
        "/cipherstream/file-transfer/1.0.0".as_bytes()
    }
}

/// File transfer request types (legacy)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileRequest {
    /// Request to transfer a file
    TransferRequest {
        filename: String,
        filesize: u64,
        encrypted: bool,
        checksum: String,
    },
    /// File data chunk
    Chunk {
        offset: u64,
        data: Vec<u8>,
    },
    /// End of file marker
    EndOfFile,
}

/// File transfer response types (legacy)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileResponse {
    /// Response to transfer request
    TransferAccepted {
        accepted: bool,
        reason: Option<String>,
    },
    /// Acknowledge receipt of a chunk
    ChunkAck {
        offset: u64,
        received_bytes: usize,
    },
    /// File transfer completed
    TransferCompleted {
        success: bool,
        error: Option<String>,
        final_checksum: Option<String>,
    },
}

/// File transfer event handler
pub struct FileTransferHandler {
    #[allow(dead_code)] // Will be used when implementing file receiving
    download_dir: String,
}

impl FileTransferHandler {
    pub fn new(download_dir: String) -> Self {
        Self { download_dir }
    }

    /// Handle an incoming file transfer request
    pub async fn handle_request(
        &self,
        peer: PeerId,
        request: FileRequest,
    ) -> Result<FileResponse, Box<dyn std::error::Error + Send + Sync>> {
        match request {
            FileRequest::TransferRequest { filename, filesize, encrypted: _, checksum: _ } => {
                println!("📥 Received file transfer request from {}: {} ({} bytes)", 
                    peer, filename, filesize);
                
                // Check if we want to accept this file
                let accept = true; // Always accept for now
                
                let response = FileResponse::TransferAccepted {
                    accepted: accept,
                    reason: None,
                };
                
                Ok(response)
            }
            FileRequest::Chunk { offset, data } => {
                // Handle incoming file chunk
                println!("📦 Received chunk from {} at offset {}, size: {} bytes", 
                    peer, offset, data.len());
                
                // Acknowledge chunk receipt
                let response = FileResponse::ChunkAck {
                    offset,
                    received_bytes: data.len(),
                };
                
                Ok(response)
            }
            FileRequest::EndOfFile => {
                // File transfer complete
                println!("✅ File transfer completed from {}", peer);
                
                let response = FileResponse::TransferCompleted {
                    success: true,
                    error: None,
                    final_checksum: None, // Should calculate checksum here
                };
                
                Ok(response)
            }
        }
    }

    /// Send a file to a peer
    pub async fn send_file(
        &self,
        peer: PeerId,
        file_path: &Path,
        encrypt: bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Open file
        let file = File::open(file_path).await?;
        
        // Get file metadata
        let metadata = file.metadata().await?;
        let filesize = metadata.len();
        let filename = file_path.file_name()
            .ok_or("Invalid filename")?
            .to_string_lossy()
            .to_string();
        
        // Calculate checksum
        let _checksum = if encrypt {
            // For encrypted files, we'd use the crypto module here
            "encrypted-checksum".to_string()
        } else {
            "plaintext-checksum".to_string()
        };
        
        println!("📤 Sending file to {}: {} ({} bytes)", peer, filename, filesize);
        
        // Create progress bar
        let pb = ProgressBar::new(filesize);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );
        
        // TODO: Send initial transfer request
        // TODO: Start sending chunks
        // TODO: Send EOF marker
        
        pb.finish_with_message(format!("✅ File {} sent successfully", filename));
        
        Ok(())
    }
}

/// Handle behavior events related to file transfers
pub async fn handle_behavior_event(
    swarm: &mut Swarm<Behavior>,
    event: request_handler::Event<ProtocolRequest, ProtocolResponse>,
) -> Result<(), Box<dyn Error>> {
    match event {
        request_handler::Event::Message { peer, message, connection_id: _ } => {
            match message {
                request_handler::Message::Request { request, channel, .. } => {
                    println!("📥 Received file transfer request from {}", peer);
                    
                    // Handle different request types
                    match request {
                        ProtocolRequest::FileData { filename, encrypted, data } => {
                            println!("📄 Received FileData: {} ({} bytes), encrypted: {}", 
                                     filename, data.len(), encrypted);
                            
                            // TODO: Save file, decrypt if needed
                            println!("💾 Saving file (placeholder)... {}", filename);
                            // Simulate saving
                            let save_success = true; 
                            let error_msg = if save_success { None } else { Some("Failed to save file".to_string()) };
                            
                            let response = ProtocolResponse::FileReceived { 
                                success: save_success, 
                                error: error_msg, 
                            };
                            
                            // Send response
                            let _ = swarm.behaviour_mut().file_transfer.send_response(channel, response);
                        }
                        // Removed other variants as they are no longer defined
                    }
                }
                request_handler::Message::Response { request_id, response } => {
                    println!("📤 Received file transfer response: {:?}, Request ID: {:?}", response, request_id);
                    // Handle FileReceived if needed (e.g., confirm sender side)
                    match response {
                         ProtocolResponse::FileReceived { success, .. } => {
                            println!("✅ Received confirmation: Success = {}", success);
                         },
                         // Remove other variants
                    }
                }
            }
        }
        request_handler::Event::OutboundFailure { peer, request_id, error, connection_id: _ } => {
             println!("❌ Outbound file transfer request failed to {}: {:?}, request ID: {:?}", 
                        peer, error, request_id);
        }
        request_handler::Event::InboundFailure { peer, request_id, error, connection_id: _ } => {
            println!("❌ Inbound file transfer request failed from {}: {:?}, request ID: {:?}", 
                        peer, error, request_id);
        }
        request_handler::Event::ResponseSent { peer, request_id, .. } => {
             println!("✅ Response sent to {}, Request ID: {:?}", peer, request_id);
        }
    }
    Ok(())
}

// Helper function to start a file transfer to a peer
pub async fn send_file_to_peer(
    swarm: &mut Swarm<Behavior>,
    peer_id: &PeerId,
    file_path: &str,
    encrypted: bool,
) -> Result<OutboundRequestId, Box<dyn Error>> {
    println!("📤 Initiating file transfer (single message) to {}", peer_id);
    
    // Check if the file exists
    let path = Path::new(file_path);
    if !path.exists() || !path.is_file() {
        return Err(format!("File not found: {}", file_path).into());
    }
    
    // Get file metadata
    let metadata = tokio::fs::metadata(file_path).await?;
    let file_size = metadata.len();
    let file_name = path.file_name()
        .ok_or("Invalid file name")?
        .to_str()
        .ok_or("Invalid file name encoding")?
        .to_string();
    
    // Read the entire file content
    let file_content = tokio::fs::read(file_path).await?;
    println!("📖 Read {} bytes from file {}", file_content.len(), file_name);
    
    // Create transfer request with full data
    let request = ProtocolRequest::FileData {
        filename: file_name.clone(),
        encrypted,
        data: file_content,
    };
    
    // Send the request
    let request_id = swarm.behaviour_mut().file_transfer.send_request(peer_id, request);
    println!("📤 Sent file data for '{}', request ID: {:?}", file_name, request_id);
    
    Ok(request_id)
}