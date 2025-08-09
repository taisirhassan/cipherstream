use crate::core::traits::Configuration;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub data_directory: String,
    pub download_directory: String,
    pub default_port: u16,
    pub max_concurrent_transfers: usize,
    pub chunk_size: usize,
    pub network: NetworkConfig,
    pub security: SecurityConfig,
}

/// Network-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub listen_addresses: Vec<String>,
    pub bootstrap_peers: Vec<String>,
    pub connection_timeout_seconds: u64,
    pub keep_alive_interval_seconds: u64,
    pub max_connections: usize,
}

/// Security-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub enable_encryption: bool,
    pub key_rotation_interval_hours: u64,
    pub max_file_size_mb: u64,
    pub allowed_file_extensions: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let data_dir = format!("{}/.cipherstream", home);

        Self {
            data_directory: data_dir.clone(),
            download_directory: format!("{}/downloads", data_dir),
            default_port: 8000,
            max_concurrent_transfers: 10,
            chunk_size: 1024 * 1024, // 1MB
            network: NetworkConfig {
                listen_addresses: vec![
                    "/ip4/0.0.0.0/tcp/0".to_string(),
                    "/ip6/::/tcp/0".to_string(),
                ],
                bootstrap_peers: vec![],
                connection_timeout_seconds: 30,
                keep_alive_interval_seconds: 60,
                max_connections: 100,
            },
            security: SecurityConfig {
                enable_encryption: true,
                key_rotation_interval_hours: 24,
                max_file_size_mb: 1024, // 1GB
                allowed_file_extensions: vec![
                    "txt".to_string(),
                    "pdf".to_string(),
                    "jpg".to_string(),
                    "png".to_string(),
                    "doc".to_string(),
                    "docx".to_string(),
                    "zip".to_string(),
                ],
            },
        }
    }
}

impl AppConfig {
    /// Load configuration from file or create default
    pub fn load_or_default(config_path: Option<&str>) -> Self {
        if let Some(config) = config_path
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|content| serde_json::from_str(&content).ok())
        {
            return config;
        }
        Self::default()
    }

    /// Save configuration to file
    pub fn save_to_file(&self, config_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    /// Get the data directory as PathBuf
    pub fn data_dir_path(&self) -> PathBuf {
        PathBuf::from(&self.data_directory)
    }

    /// Get the download directory as PathBuf
    pub fn download_dir_path(&self) -> PathBuf {
        PathBuf::from(&self.download_directory)
    }

    /// Ensure all directories exist
    pub fn ensure_directories(&self) -> Result<(), std::io::Error> {
        std::fs::create_dir_all(&self.data_directory)?;
        std::fs::create_dir_all(&self.download_directory)?;
        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.chunk_size == 0 {
            return Err("Chunk size must be greater than 0".into());
        }

        if self.max_concurrent_transfers == 0 {
            return Err("Max concurrent transfers must be greater than 0".into());
        }

        if self.default_port == 0 {
            return Err("Default port must be greater than 0".into());
        }

        // Validate network config
        if self.network.max_connections == 0 {
            return Err("Max connections must be greater than 0".into());
        }

        Ok(())
    }
}

impl Configuration for AppConfig {
    fn get_data_directory(&self) -> &str {
        &self.data_directory
    }

    fn get_download_directory(&self) -> &str {
        &self.download_directory
    }

    fn get_default_port(&self) -> u16 {
        self.default_port
    }

    fn get_max_concurrent_transfers(&self) -> usize {
        self.max_concurrent_transfers
    }

    fn get_chunk_size(&self) -> usize {
        self.chunk_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        config.validate().expect("Default config should be valid");
        assert!(config.chunk_size > 0);
        assert!(config.max_concurrent_transfers > 0);
        assert!(config.default_port > 0);
    }

    #[test]
    fn test_config_serialization() {
        let config = AppConfig::default();
        let json = serde_json::to_string(&config).expect("Should serialize");
        let _deserialized: AppConfig = serde_json::from_str(&json).expect("Should deserialize");
    }
}
