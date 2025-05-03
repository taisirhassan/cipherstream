pub mod handler;
pub mod types;
pub mod request_handler;

pub use handler::FileTransferHandler;
pub use types::{FileInfo, FileRequest, FileResponse, TransferProgress, 
                ProtocolRequest, ProtocolResponse};
pub use request_handler::{FileTransferProtocol, FileTransferCodec,
                         create_request_response, send_file_request, send_file_response};

use std::io;
use libp2p::PeerId;
use tokio::{fs::File, io::{AsyncWriteExt, AsyncSeekExt}};
use std::collections::HashMap;

/// The size of the chunks used to send files
#[allow(dead_code)]
const CHUNK_SIZE: usize = 1024 * 64; // 64KB

/// File transfer manager for handling file transfers
pub struct FileTransferManager {
    download_dir: String,
    active_transfers: HashMap<String, FileTransferState>,
}

enum FileTransferState {
    Sending {
        #[allow(dead_code)]
        file_path: String,
        offset: u64,
        #[allow(dead_code)]
        size: u64,
    },
    Receiving {
        file_path: String,
        offset: u64,
        size: u64,
        writer: Option<File>,
    },
}

impl FileTransferManager {
    pub fn new(download_dir: String) -> Self {
        std::fs::create_dir_all(&download_dir).unwrap_or_else(|e| {
            println!("Warning: Could not create download directory: {}", e);
        });
        
        Self {
            download_dir,
            active_transfers: HashMap::new(),
        }
    }

    /// Initialize sending a file to a peer
    pub async fn send_file(
        &mut self,
        transfer_id: String,
        file_path: String,
        size: u64,
    ) {
        // Add to active transfers
        self.active_transfers.insert(
            transfer_id,
            FileTransferState::Sending {
                file_path,
                offset: 0,
                size,
            },
        );
    }

    /// Handle a file request
    pub async fn handle_file_request(
        &mut self,
        peer: PeerId,
        transfer_id: String,
        file_name: String,
        file_size: u64,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        println!("📥 Received file transfer request from {}: {} ({} bytes)", 
            peer, file_name, file_size);
        
        // Create the file path in the download directory
        let file_path = format!("{}/{}", self.download_dir, file_name);
        
        // Check if we want to accept this file
        let accept = true; // Always accept for now
        
        if accept {
            // Create or truncate the file
            let file = File::create(&file_path).await?;
            file.set_len(0).await?;
            
            // Add to active transfers
            self.active_transfers.insert(
                transfer_id,
                FileTransferState::Receiving {
                    file_path: file_path.clone(),
                    offset: 0,
                    size: file_size,
                    writer: Some(file),
                },
            );
        }
        
        Ok(accept)
    }

    /// Process a chunk of file data
    pub async fn process_chunk(
        &mut self,
        transfer_id: &str,
        offset: u64,
        data: Vec<u8>,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        // Find the transfer
        if let Some(FileTransferState::Receiving { file_path: _, offset: current_offset, size: _, writer: Some(file) }) = 
            self.active_transfers.get_mut(transfer_id) {
            
            // Verify the offset
            if *current_offset != offset {
                return Err("Invalid offset".into());
            }
            
            // Write the data
            file.seek(io::SeekFrom::Start(offset)).await?;
            file.write_all(&data).await?;
            
            // Update the offset
            *current_offset += data.len() as u64;
            
            // Return received size
            Ok(data.len())
        } else {
            Err("No active transfer for this request".into())
        }
    }

    /// Complete a file transfer
    pub async fn complete_transfer(
        &mut self,
        transfer_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(FileTransferState::Receiving { file_path, offset, size, writer }) = 
            self.active_transfers.remove(transfer_id) {
            
            // Close the file if it exists
            if let Some(mut file) = writer {
                file.flush().await?;
                // Dropping the file will close it
            }
            
            println!("✅ File saved to {}", file_path);
            println!("📊 Received {} of {} bytes", offset, size);
            
            Ok(())
        } else {
            Err("No active transfer for this request".into())
        }
    }

    /// Process a transfer acceptance response 
    pub async fn process_transfer_accepted(
        &mut self,
        transfer_id: &str,
        accepted: bool,
        reason: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if accepted {
            println!("✅ Transfer accepted: {}", transfer_id);
            Ok(())
        } else {
            println!("❌ Transfer rejected: {}, reason: {:?}", transfer_id, reason);
            self.active_transfers.remove(transfer_id);
            Err(format!("Transfer rejected: {:?}", reason).into())
        }
    }

    /// Process a chunk acknowledgement
    pub async fn process_chunk_ack(
        &mut self,
        transfer_id: &str,
        offset: u64,
        received_bytes: usize,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(FileTransferState::Sending { file_path: _, offset: current_offset, size: _ }) = 
            self.active_transfers.get_mut(transfer_id) {
            
            // Verify the offset
            if *current_offset != offset {
                return Err("Invalid offset acknowledgement".into());
            }
            
            // Update the offset
            *current_offset += received_bytes as u64;
            
            println!("📊 Chunk acknowledged: {} bytes at offset {}", received_bytes, offset);
            Ok(())
        } else {
            Err("No active transfer for this request".into())
        }
    }

    /// Process a transfer completion
    pub async fn process_transfer_completed(
        &mut self,
        transfer_id: &str,
        success: bool,
        error: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if success {
            println!("✅ Transfer completed successfully: {}", transfer_id);
        } else {
            println!("❌ Transfer failed: {}, error: {:?}", transfer_id, error);
            return Err(format!("Transfer failed: {:?}", error).into());
        }
        
        // Remove the transfer
        self.active_transfers.remove(transfer_id);
        Ok(())
    }
} 