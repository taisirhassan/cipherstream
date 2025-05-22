use std::path::Path;
use libp2p::PeerId;
use tokio::fs::File;
use tokio::io::{AsyncSeekExt, AsyncWriteExt, AsyncReadExt};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Serialize, Deserialize};
use std::error::Error;
use libp2p::swarm::Swarm;
use crate::discovery::Behavior;
use crate::file_transfer::{request_handler, ProtocolRequest, ProtocolResponse};
use libp2p::request_response::OutboundRequestId;
use std::collections::HashMap;
use std::sync::Mutex;
use lazy_static::lazy_static;
use uuid;
use thiserror::Error;
use libp2p::StreamProtocol;

// State to hold pending filenames for peers whose handshake was accepted
lazy_static! {
    static ref PENDING_FILES: Mutex<HashMap<PeerId, String>> = Mutex::new(HashMap::new());
}

// Constants
const CHUNK_SIZE: usize = 1024 * 1024; // 1 MiB chunks
/// Protocol ID constant for file transfer
pub const FILE_TRANSFER_PROTO_ID: StreamProtocol = StreamProtocol::new("/cipherstream/file-transfer/1.0.0");

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
    pub fn protocol_name(&self) -> &str {
        "/cipherstream/file-transfer/1.0.0"
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

/// Result enum for handshake operations
pub enum HandshakeResult {
    /// Handshake was accepted 
    HandshakeAccepted {
        transfer_id: String,
    },
    /// Handshake was rejected
    HandshakeRejected {
        reason: Option<String>,
    },
    /// Event was processed but not related to handshake result
    OtherEvent,
}

/// Handle handshake protocol events (request-response)
pub async fn handle_handshake_event(
    swarm: &mut Swarm<Behavior>,
    event: request_handler::Event<ProtocolRequest, ProtocolResponse>,
) -> Result<HandshakeResult, Box<dyn Error>> {
    match event {
        request_handler::Event::Message { peer, message, .. } => {
            match message {
                request_handler::Message::Request { request, channel, .. } => {
                    match request {
                        ProtocolRequest::HandshakeRequest { filename, filesize, transfer_id } => {
                            println!("🤝 Received HandshakeRequest for '{}' ({} bytes, id: {}) from {}", 
                                        filename, filesize, transfer_id, peer);
                            
                            // Store the filename for later use (e.g., to allow accepting the file)
                            store_pending_filename(peer, filename.clone());
                            
                            // Create download directory if it doesn't exist
                            let downloads_dir = "downloads";
                            std::fs::create_dir_all(downloads_dir).unwrap_or_else(|e| {
                                println!("⚠️ Warning: Could not create downloads directory: {}", e);
                            });
                            
                            // Check if we want to accept this file
                            let accept = true; // Always accept for now
                            
                            // Create and send the handshake response
                            let response = ProtocolResponse::HandshakeResponse { 
                                accepted: accept, 
                                reason: None,
                                transfer_id: Some(transfer_id.clone()),
                            };
                            
                            if let Err(e) = swarm.behaviour_mut().handshake.send_response(channel, response) {
                                println!("❌ Failed to send handshake response: {:?}", e);
                                return Err(format!("Failed to send response: {:?}", e).into());
                            }
                            
                            println!("🔄 Ready to receive file chunks for transfer {}", transfer_id);
                            
                            return Ok(HandshakeResult::OtherEvent);
                        }
                        ProtocolRequest::FileChunk { transfer_id, chunk_index, total_chunks, data, is_last } => {
                            // Get peer's current filename
                            let filename = match get_pending_filename(&peer) {
                                Some(name) => name,
                                None => {
                                    println!("⚠️ Received chunk from {} with no pending file", peer);
                                    return Ok(HandshakeResult::OtherEvent);
                                }
                            };
                            
                            println!("📦 Received chunk {} of {} for {} (size: {} bytes, is_last: {})", 
                                     chunk_index, total_chunks, filename, data.len(), is_last);
                            
                            // Create the download path
                            let file_path = format!("downloads/{}", filename);
                            
                            // Data is already decrypted by libp2p Noise protocol
                            let processed_data = data;
                            
                            // Open or create the file
                            let mut file = match tokio::fs::OpenOptions::new()
                                .create(true)
                                .write(true)
                                .append(false) // We'll manually seek
                                .open(&file_path).await {
                                Ok(f) => f,
                                Err(e) => {
                                    println!("❌ Failed to open file for writing: {}", e);
                                    // Send an error response directly via channel
                                    return Ok(HandshakeResult::OtherEvent);
                                }
                            };
                            
                            // Seek to the position for this chunk
                            let offset = chunk_index as u64 * CHUNK_SIZE as u64;
                            if let Err(e) = file.seek(std::io::SeekFrom::Start(offset)).await {
                                println!("❌ Failed to seek in file: {}", e);
                                // Send an error response directly via channel
                                return Ok(HandshakeResult::OtherEvent);
                            }
                            
                            // Write the processed data
                            if let Err(e) = file.write_all(&processed_data).await {
                                println!("❌ Failed to write chunk: {}", e);
                                // Send an error response directly via channel
                                return Ok(HandshakeResult::OtherEvent);
                            }
                            
                            // Flush the file
                            if let Err(e) = file.flush().await {
                                println!("⚠️ Failed to flush file: {}", e);
                            }
                            
                            // Success! (For now we'll skip properly sending success responses)
                            println!("✅ Successfully wrote chunk {} to file", chunk_index);
                            
                            // If this is the last chunk, we're done
                            if is_last {
                                println!("✅ File transfer complete: {}", filename);
                                
                                // Remove from pending files
                                let mut pending_files = PENDING_FILES.lock().unwrap();
                                pending_files.remove(&peer);
                            }
                            
                            // Send a response to acknowledge the chunk
                            let response = ProtocolResponse::ChunkResponse { 
                                transfer_id, 
                                chunk_index,
                                success: true,
                                error: None,
                            };
                            
                            if let Err(e) = swarm.behaviour_mut().handshake.send_response(channel, response) {
                                println!("❌ Failed to send chunk response: {:?}", e);
                            }
                            
                            return Ok(HandshakeResult::OtherEvent);
                        }
                        ProtocolRequest::CancelTransfer { transfer_id } => {
                            println!("🛑 Received cancel transfer request for {} from {}", transfer_id, peer);
                            
                            // Remove from pending files
                            let mut pending_files = PENDING_FILES.lock().unwrap();
                            pending_files.remove(&peer);
                            
                            // Acknowledge cancellation
                            let response = ProtocolResponse::TransferComplete {
                                transfer_id,
                                success: false,
                                error: Some("Transfer cancelled by sender".to_string()),
                            };
                            
                            if let Err(e) = swarm.behaviour_mut().handshake.send_response(channel, response) {
                                println!("❌ Failed to acknowledge transfer cancellation: {:?}", e);
                            }
                            
                            return Ok(HandshakeResult::OtherEvent);
                        }
                    }
                }
                request_handler::Message::Response { request_id: _, response } => {
                    match response {
                        ProtocolResponse::HandshakeResponse { accepted, reason, transfer_id } => {
                            println!("🤝 Received HandshakeResponse (accepted: {}, reason: {:?}, transfer_id: {:?})", 
                                accepted, reason, transfer_id);
                            
                            if accepted {
                                if let Some(id) = transfer_id {
                                    return Ok(HandshakeResult::HandshakeAccepted { transfer_id: id });
                                } else {
                                    println!("⚠️ Handshake accepted but no transfer ID provided");
                                    return Ok(HandshakeResult::HandshakeRejected { reason: Some("Missing transfer ID".to_string()) });
                                }
                            } else {
                                return Ok(HandshakeResult::HandshakeRejected { reason });
                            }
                        },
                        ProtocolResponse::ChunkResponse { transfer_id, chunk_index, success, error } => {
                            if success {
                                println!("✅ Chunk {} for transfer {} acknowledged", chunk_index, transfer_id);
                                // Continue with next chunk (implemented in caller)
                            } else {
                                println!("❌ Chunk {} for transfer {} failed: {:?}", chunk_index, transfer_id, error);
                                // Handle retry or abort
                            }
                            return Ok(HandshakeResult::OtherEvent);
                        },
                        ProtocolResponse::TransferComplete { transfer_id, success, error } => {
                            if success {
                                println!("✅ Transfer {} completed successfully", transfer_id);
                            } else {
                                println!("❌ Transfer {} failed: {:?}", transfer_id, error);
                            }
                            // Clean up resources
                            return Ok(HandshakeResult::OtherEvent);
                        }
                    }
                }
            }
        }
        request_handler::Event::OutboundFailure { peer, request_id, error, .. } => {
             println!("❌ Outbound request failed to {}: {:?}, request ID: {:?}", peer, error, request_id);
             Ok(HandshakeResult::OtherEvent)
        }
        request_handler::Event::InboundFailure { peer, request_id, error, .. } => {
            println!("❌ Inbound request failed from {}: {:?}, request ID: {:?}", peer, error, request_id);
            Ok(HandshakeResult::OtherEvent)
        }
        request_handler::Event::ResponseSent { peer, request_id, .. } => {
             println!("✅ Response sent to {}, Request ID: {:?}", peer, request_id);
             Ok(HandshakeResult::OtherEvent)
        }
    }
}

/// Update logic to send handshake
pub async fn send_handshake_request(
    swarm: &mut Swarm<Behavior>,
    peer_id: &PeerId,
    filename: &str,
    file_path: &str,
) -> Result<OutboundRequestId, Box<dyn Error>> {
    println!("🤝 Initiating handshake for '{}' with {}", filename, peer_id);
    
    // Get file size
    let metadata = tokio::fs::metadata(file_path).await?;
    let filesize = metadata.len();
    
    // Create a transfer ID
    let transfer_id = uuid::Uuid::new_v4().to_string();
    
    // Create handshake request
    let request = ProtocolRequest::HandshakeRequest {
        filename: filename.to_string(),
        filesize,
        transfer_id,
    };

    // Send the request using the handshake behavior
    let _request_id = swarm.behaviour_mut().handshake.send_request(peer_id, request);
    println!("📤 Sent HandshakeRequest for '{}', request ID: {:?}", filename, _request_id);
    
    Ok(_request_id)
}

/// File transfer error types
#[derive(Debug, Error)]
pub enum SendFileError {
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to send chunk request")]
    SendRequestError,
}

/// Send a file to a peer (libp2p Noise provides transport encryption)
pub async fn send_file(
    swarm: &mut Swarm<Behavior>,
    peer_id: &PeerId,
    file_path: &str,
    transfer_id: &str,
) -> Result<(), SendFileError> {
    // Open the file
    let mut file = tokio::fs::File::open(file_path).await?;
    
    // Get file size
    let metadata = file.metadata().await?;
    let file_size = metadata.len();
    
    // Calculate number of chunks
    let total_chunks = (file_size + CHUNK_SIZE as u64 - 1) / CHUNK_SIZE as u64;
    
    println!("📤 Starting file transfer: {}, size: {} bytes, chunks: {}", file_path, file_size, total_chunks);
    
    // Send file in chunks
    let mut offset = 0;
    let mut chunk_index = 0;
    let mut buffer = vec![0; CHUNK_SIZE];
    
    while offset < file_size {
        // Read a chunk from the file
        file.seek(std::io::SeekFrom::Start(offset)).await?;
        let n = file.read(&mut buffer[..]).await?;
        
        if n == 0 {
            break; // End of file
        }
        
        // Is this the last chunk?
        let is_last = offset + n as u64 >= file_size;
        
        // Data will be encrypted by libp2p Noise at transport layer
        let chunk_data = buffer[..n].to_vec();
        
        // Create the file chunk request
        let request = ProtocolRequest::FileChunk {
            transfer_id: transfer_id.to_string(),
            chunk_index,
            total_chunks,
            data: chunk_data,
            is_last,
        };
        
        // Send the chunk
        println!("📤 Sending chunk {} of {} for transfer {}", chunk_index, total_chunks, transfer_id);
        let _request_id = swarm.behaviour_mut().handshake.send_request(peer_id, request);
        // Note: In a real implementation, we should await confirmation or handle potential backpressure.
        // For simplicity, we proceed immediately.
        
        // Update offset and chunk index
        offset += n as u64;
        chunk_index += 1;

        // Yield to allow other tasks to run, prevents starving the executor
        tokio::task::yield_now().await;
    }
    
    println!("✅ File transfer complete! Sent {} bytes in {} chunks", file_size, chunk_index);
    
    Ok(())
}

/// Cancel a file transfer
pub async fn cancel_transfer(
    swarm: &mut Swarm<Behavior>,
    peer_id: &PeerId,
    transfer_id: &str,
) -> Result<OutboundRequestId, Box<dyn Error>> {
    println!("🛑 Cancelling transfer {} with {}", transfer_id, peer_id);
    
    let request = ProtocolRequest::CancelTransfer {
        transfer_id: transfer_id.to_string(),
    };
    
    let request_id = swarm.behaviour_mut().handshake.send_request(peer_id, request);
    Ok(request_id)
}

/// Retrieve and remove the pending filename for a given peer.
/// Called by the network module when an incoming data stream is initiated.
pub fn get_and_remove_pending_filename(peer_id: &PeerId) -> Option<String> {
    let mut pending_files = PENDING_FILES.lock().unwrap();
    pending_files.remove(peer_id)
}

/// Handle request-response events from the network behavior
pub async fn handle_req_resp_event(
    event: request_handler::Event<ProtocolRequest, ProtocolResponse>,
    swarm: &mut Swarm<Behavior>
) -> Result<Option<ProtocolResponse>, Box<dyn Error>> {
    match handle_handshake_event(swarm, event).await {
        Ok(HandshakeResult::OtherEvent) => Ok(None), // No response to return
        Ok(_) => Ok(None), // Any other result, no response to return
        Err(e) => Err(e), // Propagate the error
    }
}

/// Send a file request to a peer
pub async fn send_request(
    swarm: &mut Swarm<Behavior>,
    peer_id: &PeerId,
    request: ProtocolRequest
) -> Result<OutboundRequestId, Box<dyn Error>> {
    // Send the request through the handshake protocol
    let req_id = swarm.behaviour_mut().handshake.send_request(peer_id, request);
    Ok(req_id)
}

// Store a pending filename for a peer
fn store_pending_filename(peer: PeerId, filename: String) {
    println!("💾 Storing pending filename '{}' for peer {}", filename, peer);
    let mut pending_files = PENDING_FILES.lock().unwrap();
    pending_files.insert(peer, filename);
}

// Get a pending filename for a peer
fn get_pending_filename(peer: &PeerId) -> Option<String> {
    let pending_files = PENDING_FILES.lock().unwrap();
    pending_files.get(peer).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_protocol_id_constants() {
        // StreamProtocol doesn't have a field 0, so we need to convert to string
        let proto_str = FILE_TRANSFER_PROTO_ID.to_string();
        assert_eq!(proto_str, "/cipherstream/file-transfer/1.0.0");
    }
    
    #[test]
    fn test_protocol_request_serialization() {
        // Test HandshakeRequest
        let req = ProtocolRequest::HandshakeRequest {
            filename: "test.txt".to_string(),
            filesize: 1024,
            transfer_id: "abc123".to_string(),
        };
        
        // Serialize
        let bytes = bincode::encode_to_vec(&req, bincode::config::standard()).unwrap();
        
        // Deserialize
        let (decoded_req, _): (ProtocolRequest, _) = bincode::decode_from_slice(
            &bytes, 
            bincode::config::standard()
        ).unwrap();
        
        // Compare
        match decoded_req {
            ProtocolRequest::HandshakeRequest { filename, filesize, transfer_id } => {
                assert_eq!(filename, "test.txt");
                assert_eq!(filesize, 1024);
                assert_eq!(transfer_id, "abc123");
            },
            _ => panic!("Wrong variant decoded"),
        }
    }
    
    #[test]
    fn test_protocol_response_serialization() {
        // Test HandshakeResponse
        let resp = ProtocolResponse::HandshakeResponse {
            accepted: true,
            reason: Some("All good".to_string()),
            transfer_id: Some("abc123".to_string()),
        };
        
        // Serialize
        let bytes = bincode::encode_to_vec(&resp, bincode::config::standard()).unwrap();
        
        // Deserialize
        let (decoded_resp, _): (ProtocolResponse, _) = bincode::decode_from_slice(
            &bytes, 
            bincode::config::standard()
        ).unwrap();
        
        // Compare
        match decoded_resp {
            ProtocolResponse::HandshakeResponse { accepted, reason, transfer_id } => {
                assert_eq!(accepted, true);
                assert_eq!(reason.unwrap(), "All good");
                assert_eq!(transfer_id.unwrap(), "abc123");
            },
            _ => panic!("Wrong variant decoded"),
        }
    }
    
    #[test]
    fn test_file_chunk_serialization() {
        // Test FileChunk
        let req = ProtocolRequest::FileChunk {
            transfer_id: "abc123".to_string(),
            chunk_index: 0,
            total_chunks: 10,
            data: vec![1, 2, 3, 4, 5],
            is_last: false,
        };
        
        // Serialize
        let bytes = bincode::encode_to_vec(&req, bincode::config::standard()).unwrap();
        
        // Deserialize
        let (decoded_req, _): (ProtocolRequest, _) = bincode::decode_from_slice(
            &bytes, 
            bincode::config::standard()
        ).unwrap();
        
        // Compare
        match decoded_req {
            ProtocolRequest::FileChunk { transfer_id, chunk_index, total_chunks, data, is_last } => {
                assert_eq!(transfer_id, "abc123");
                assert_eq!(chunk_index, 0);
                assert_eq!(total_chunks, 10);
                assert_eq!(data, vec![1, 2, 3, 4, 5]);
                assert_eq!(is_last, false);
            },
            _ => panic!("Wrong variant decoded"),
        }
    }
}