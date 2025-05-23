use std::sync::Arc;
use crate::core::traits::*;
use crate::infrastructure::{repositories::*, config::AppConfig, UtilityService};

/// Application service that provides dependency injection and orchestrates the system
pub struct ApplicationService {
    pub config: Arc<AppConfig>,
    // pub use_cases: Arc<super::UseCases>,
    
    // Domain services
    // pub transfer_service: Arc<TransferDomainService>,
    // pub peer_service: Arc<PeerDomainService>,
    // pub file_service: Arc<FileDomainService>,
    
    // Repositories
    pub file_repository: Arc<dyn FileRepository>,
    pub transfer_repository: Arc<dyn TransferRepository>,
    pub peer_repository: Arc<dyn PeerRepository>,
}

impl ApplicationService {
    /// Create a new application service with all dependencies wired up
    pub async fn new(config: AppConfig) -> Result<Self, Box<dyn std::error::Error>> {
        // Validate and prepare configuration
        config.validate()?;
        config.ensure_directories()?;
        
        let config = Arc::new(config);

        // Create repositories
        let file_repository = RepositoryBuilder::build_file_repository();
        let transfer_repository = RepositoryBuilder::build_transfer_repository();
        let peer_repository = RepositoryBuilder::build_peer_repository();

        Ok(Self {
            config,
            file_repository,
            transfer_repository,
            peer_repository,
        })
    }

    /// Get the application configuration
    pub fn config(&self) -> &AppConfig {
        &self.config
    }
}

/// File system implementation of FileService
pub struct FileSystemService {
    _config: Arc<AppConfig>,
}

impl FileSystemService {
    pub fn new(config: Arc<AppConfig>) -> Self {
        Self { 
            _config: config,
        }
    }
}

#[async_trait::async_trait]
impl FileService for FileSystemService {
    async fn add_file(&self, path: &str) -> DomainResult<crate::core::domain::File> {
        let (name, size) = self.get_file_metadata(path).await?;
        let hash = self.calculate_file_hash(path).await?;
        
        Ok(crate::core::domain::File {
            id: crate::core::domain::FileId::new(),
            name,
            size,
            hash,
            path: path.to_string(),
            created_at: std::time::SystemTime::now(),
            modified_at: None,
        })
    }

    async fn calculate_file_hash(&self, path: &str) -> DomainResult<String> {
        UtilityService::sha256_file(path)
            .await
            .map_err(|e| e.into())
    }

    async fn get_file_metadata(&self, path: &str) -> DomainResult<(String, u64)> {
        let path_buf = std::path::Path::new(path);
        let metadata = tokio::fs::metadata(path).await?;
        
        let name = path_buf
            .file_name()
            .ok_or("Invalid filename")?
            .to_string_lossy()
            .to_string();
        
        Ok((name, metadata.len()))
    }

    async fn read_file_chunk(&self, path: &str, offset: u64, size: usize) -> DomainResult<Vec<u8>> {
        use tokio::io::{AsyncReadExt, AsyncSeekExt};
        
        let mut file = tokio::fs::File::open(path).await?;
        file.seek(std::io::SeekFrom::Start(offset)).await?;
        
        let mut buffer = vec![0u8; size];
        let bytes_read = file.read(&mut buffer).await?;
        buffer.truncate(bytes_read);
        
        Ok(buffer)
    }

    async fn write_file_chunk(&self, path: &str, offset: u64, data: &[u8]) -> DomainResult<()> {
        use tokio::io::{AsyncWriteExt, AsyncSeekExt};
        
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(path)
            .await?;
        
        file.seek(std::io::SeekFrom::Start(offset)).await?;
        file.write_all(data).await?;
        file.flush().await?;
        
        Ok(())
    }
} 