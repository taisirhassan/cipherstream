use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use libp2p::PeerId;
use tokio::fs::{self, File};
use tokio::io::{AsyncReadExt, AsyncWriteExt, AsyncSeekExt};

use crate::file_transfer::types::{FileInfo, FileRequest, FileResponse, TransferProgress};
use crate::utils;

const CHUNK_SIZE: usize = 64 * 1024; // 64KB chunks

/// Handler for file transfer operations
pub struct FileTransferHandler {
    /// Directory where transferred files are stored
    download_dir: PathBuf,
    /// Directory where shared files are stored
    upload_dir: PathBuf,
    /// Active transfers map
    active_transfers: Arc<Mutex<HashMap<String, TransferProgress>>>,
    /// File list that we're sharing
    shared_files: Arc<Mutex<Vec<FileInfo>>>,
}

impl FileTransferHandler {
    /// Create a new FileTransferHandler
    pub fn new(download_dir: PathBuf, upload_dir: PathBuf) -> Self {
        // Create directories if they don't exist
        tokio::runtime::Handle::current().block_on(async {
            fs::create_dir_all(&download_dir).await.unwrap_or_else(|e| {
                eprintln!("Failed to create download directory: {}", e);
            });
            fs::create_dir_all(&upload_dir).await.unwrap_or_else(|e| {
                eprintln!("Failed to create upload directory: {}", e);
            });
        });

        Self {
            download_dir,
            upload_dir,
            active_transfers: Arc::new(Mutex::new(HashMap::new())),
            shared_files: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Add a file to the shared files list
    pub async fn add_shared_file<P: AsRef<Path>>(&self, path: P) -> Result<FileInfo, std::io::Error> {
        let path = path.as_ref();
        let metadata = fs::metadata(path).await?;
        
        let file_name = path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
            
        let file_size = metadata.len();
        
        // Calculate hash
        let hash = utils::sha256_file(path).await?;
        
        let file_info = FileInfo {
            id: utils::generate_id(),
            name: file_name,
            size: file_size,
            hash,
            modified: metadata.modified().ok(),
            mime_type: None, // Could be determined by extension
        };
        
        // Add to shared files
        self.shared_files.lock().unwrap().push(file_info.clone());
        
        Ok(file_info)
    }

    /// Handle a file request
    pub async fn handle_request(&self, _peer_id: PeerId, request: FileRequest) -> FileResponse {
        match request {
            FileRequest::ListFiles => {
                // Return list of shared files
                let files = self.shared_files.lock().unwrap().clone();
                FileResponse::FileList { files }
            },
            
            FileRequest::DownloadFile { file_id, range: _ } => {
                // Find the file
                let file = {
                    let shared_files = self.shared_files.lock().unwrap();
                    shared_files.iter()
                        .find(|f| f.id == file_id)
                        .cloned()
                };
                
                match file {
                    Some(file_info) => {
                        let transfer_id = utils::generate_id();
                        let file_path = self.upload_dir.join(&file_info.name);
                        
                        // Validate that file exists and is accessible
                        match fs::metadata(&file_path).await {
                            Ok(metadata) => {
                                let total_chunks = (metadata.len() + CHUNK_SIZE as u64 - 1) / CHUNK_SIZE as u64;
                                
                                // Register transfer
                                let progress = TransferProgress {
                                    transfer_id: transfer_id.clone(),
                                    file_info: file_info.clone(),
                                    bytes_transferred: 0,
                                    chunks_transferred: 0,
                                    total_chunks,
                                    total_bytes: file_info.size,
                                    percentage: 0.0,
                                    start_time: SystemTime::now(),
                                    local_path: file_path.to_string_lossy().to_string(),
                                };
                                
                                self.active_transfers.lock().unwrap().insert(transfer_id.clone(), progress);
                                
                                FileResponse::DownloadAccepted {
                                    transfer_id,
                                    file_info,
                                    total_chunks,
                                }
                            },
                            Err(e) => FileResponse::DownloadRejected {
                                file_id,
                                reason: format!("File not accessible: {}", e),
                            },
                        }
                    },
                    None => FileResponse::DownloadRejected {
                        file_id,
                        reason: "File not found".to_string(),
                    },
                }
            },
            
            FileRequest::UploadFile { file_info } => {
                // Generate a transfer ID
                let transfer_id = utils::generate_id();
                
                // Create file path in download directory
                let file_path = self.download_dir.join(&file_info.name);
                
                // Register transfer
                let progress = TransferProgress {
                    transfer_id: transfer_id.clone(),
                    file_info: file_info.clone(),
                    bytes_transferred: 0,
                    chunks_transferred: 0,
                    total_chunks: (file_info.size + CHUNK_SIZE as u64 - 1) / CHUNK_SIZE as u64,
                    total_bytes: file_info.size,
                    percentage: 0.0,
                    start_time: SystemTime::now(),
                    local_path: file_path.to_string_lossy().to_string(),
                };
                
                self.active_transfers.lock().unwrap().insert(transfer_id.clone(), progress);
                
                FileResponse::UploadAccepted { transfer_id }
            },
            
            FileRequest::CancelTransfer { transfer_id } => {
                // Remove from active transfers
                self.active_transfers.lock().unwrap().remove(&transfer_id);
                
                FileResponse::TransferComplete { transfer_id }
            },
        }
    }
    
    /// Send a file chunk for a given transfer
    pub async fn send_chunk(&self, transfer_id: &str, chunk_index: u64) -> Result<FileResponse, std::io::Error> {
        // Get transfer info
        let transfer_info = {
            match self.active_transfers.lock().unwrap().get(transfer_id).cloned() {
                Some(info) => info,
                None => return Ok(FileResponse::Error {
                    message: format!("Transfer {} not found", transfer_id),
                }),
            }
        };
        
        // Open the file
        let mut file = File::open(&transfer_info.local_path).await?;
        
        // Calculate offset and chunk size
        let offset = chunk_index * CHUNK_SIZE as u64;
        if offset >= transfer_info.total_bytes {
            return Ok(FileResponse::Error {
                message: format!("Chunk index out of bounds: {}", chunk_index),
            });
        }
        
        // Seek to the right position
        file.seek(std::io::SeekFrom::Start(offset)).await?;
        
        // Determine chunk size (could be smaller for the last chunk)
        let chunk_size = std::cmp::min(
            CHUNK_SIZE as u64,
            transfer_info.total_bytes - offset,
        ) as usize;
        
        // Read chunk
        let mut buffer = vec![0u8; chunk_size];
        let bytes_read = file.read_exact(&mut buffer).await?;
        
        // Update transfer progress
        {
            let mut transfers = self.active_transfers.lock().unwrap();
            if let Some(progress) = transfers.get_mut(transfer_id) {
                progress.chunks_transferred += 1;
                progress.bytes_transferred += bytes_read as u64;
            }
        }
        
        // Return chunk
        Ok(FileResponse::FileChunk {
            transfer_id: transfer_id.to_string(),
            chunk_index,
            total_chunks: transfer_info.total_chunks,
            data: buffer,
        })
    }
    
    /// Process a received file chunk
    pub async fn process_chunk(&self, chunk: FileResponse) -> Result<(), std::io::Error> {
        match chunk {
            FileResponse::FileChunk { transfer_id, chunk_index, total_chunks, data } => {
                // Get transfer info
                let transfer_info = {
                    match self.active_transfers.lock().unwrap().get(&transfer_id).cloned() {
                        Some(info) => info,
                        None => return Err(std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            format!("Transfer {} not found", transfer_id),
                        )),
                    }
                };
                
                // Open file for writing (create or append)
                let mut file = File::options()
                    .write(true)
                    .create(true)
                    .open(&transfer_info.local_path)
                    .await?;
                
                // Seek to the right position
                let offset = chunk_index * CHUNK_SIZE as u64;
                file.seek(std::io::SeekFrom::Start(offset)).await?;
                
                // Write chunk
                file.write_all(&data).await?;
                
                // Update transfer progress
                {
                    let mut transfers = self.active_transfers.lock().unwrap();
                    if let Some(progress) = transfers.get_mut(&transfer_id) {
                        progress.chunks_transferred += 1;
                        progress.bytes_transferred += data.len() as u64;
                        
                        // If all chunks received, add to shared files
                        if progress.chunks_transferred >= total_chunks {
                            if let Ok(metadata) = fs::metadata(&progress.local_path).await {
                                let completed_file = FileInfo {
                                    id: utils::generate_id(),
                                    name: progress.file_info.name.clone(),
                                    size: metadata.len(),
                                    hash: progress.file_info.hash.clone(),
                                    modified: metadata.modified().ok(),
                                    mime_type: progress.file_info.mime_type.clone(),
                                };
                                
                                // Add to shared files
                                self.shared_files.lock().unwrap().push(completed_file);
                            }
                        }
                    }
                }
                
                Ok(())
            },
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Expected file chunk response",
            )),
        }
    }
    
    /// Get the list of active transfers
    pub fn get_active_transfers(&self) -> Vec<TransferProgress> {
        self.active_transfers.lock().unwrap().values().cloned().collect()
    }
    
    /// Get the list of shared files
    pub fn get_shared_files(&self) -> Vec<FileInfo> {
        self.shared_files.lock().unwrap().clone()
    }
} 