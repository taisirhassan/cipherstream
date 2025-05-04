use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use std::path::PathBuf;

/// Default port for network connections
pub const DEFAULT_PORT: u16 = 8000;

/// Network configuration struct
pub struct NetworkConfig {
    /// Port to bind to
    pub port: u16,
    
    /// Directory for storing data
    pub data_dir: PathBuf,
    
    /// Bootstrap peers to connect to on startup
    pub bootstrap_peers: Vec<String>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        Self {
            port: DEFAULT_PORT,
            data_dir: PathBuf::from(format!("{}/.cipherstream", home)),
            bootstrap_peers: Vec::new(),
        }
    }
}

impl NetworkConfig {
    /// Create a new NetworkConfig with custom settings
    pub fn new(port: Option<u16>, data_dir: Option<PathBuf>, bootstrap_peers: Option<Vec<String>>) -> Self {
        let mut config = Self::default();
        
        if let Some(p) = port {
            config.port = p;
        }
        
        if let Some(dir) = data_dir {
            config.data_dir = dir;
        }
        
        if let Some(peers) = bootstrap_peers {
            config.bootstrap_peers = peers;
        }
        
        config
    }
    
    /// Get the socket address to bind to
    pub fn get_socket_addr(&self) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), self.port)
    }
    
    /// Get the directory for downloads
    pub fn get_downloads_dir(&self) -> PathBuf {
        self.data_dir.join("downloads")
    }
    
    /// Get the directory for key storage
    pub fn get_keys_dir(&self) -> PathBuf {
        self.data_dir.join("keys")
    }
} 