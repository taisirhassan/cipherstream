use cipherstream::network::config::{NetworkConfig, DEFAULT_PORT};
use std::net::SocketAddr;
use std::path::PathBuf;

#[test]
fn test_default_config() {
    let config = NetworkConfig::default();
    
    // Check default values
    assert_eq!(config.port, DEFAULT_PORT);
    assert!(config.data_dir.ends_with(".cipherstream"));
    assert!(config.bootstrap_peers.is_empty());
}

#[test]
fn test_custom_port_config() {
    let custom_port = 12345;
    let config = NetworkConfig::new(Some(custom_port), None, None);
    
    assert_eq!(config.port, custom_port);
    assert!(config.data_dir.ends_with(".cipherstream"));
    assert!(config.bootstrap_peers.is_empty());
}

#[test]
fn test_custom_data_dir_config() {
    let custom_dir = PathBuf::from("/tmp/test_data_dir");
    let config = NetworkConfig::new(None, Some(custom_dir.clone()), None);
    
    assert_eq!(config.port, DEFAULT_PORT);
    assert_eq!(config.data_dir, custom_dir);
    assert!(config.bootstrap_peers.is_empty());
}

#[test]
fn test_bootstrap_peers_config() {
    let peers = vec![
        "/ip4/127.0.0.1/tcp/1234/p2p/12D3KooWA6JG3XjSTG8U3WWW7Z1YUkfzBKdLpL76Vz53QFMdxfNf".to_string(),
        "/ip4/192.168.1.1/tcp/4321/p2p/12D3KooWRFC9qu1FQFnWh9vdpBx6j9KSZsANH7jJYHsZ8P2WDBCd".to_string(),
    ];
    
    let config = NetworkConfig::new(None, None, Some(peers.clone()));
    
    assert_eq!(config.port, DEFAULT_PORT);
    assert!(config.data_dir.ends_with(".cipherstream"));
    assert_eq!(config.bootstrap_peers, peers);
}

#[test]
fn test_get_socket_addr() {
    let config = NetworkConfig::new(Some(8080), None, None);
    let socket_addr = config.get_socket_addr();
    
    // Should be a local socket address with the specified port
    match socket_addr {
        SocketAddr::V4(addr) => {
            assert_eq!(addr.port(), 8080);
            assert!(addr.ip().is_unspecified() || addr.ip().is_loopback());
        },
        SocketAddr::V6(addr) => {
            assert_eq!(addr.port(), 8080);
            assert!(addr.ip().is_unspecified() || addr.ip().is_loopback());
        }
    }
}

#[test]
fn test_download_dir() {
    let custom_dir = PathBuf::from("/tmp/test_data_dir");
    let config = NetworkConfig::new(None, Some(custom_dir.clone()), None);
    
    let downloads_dir = config.get_downloads_dir();
    assert_eq!(downloads_dir, custom_dir.join("downloads"));
}

#[test]
fn test_keys_dir() {
    let custom_dir = PathBuf::from("/tmp/test_data_dir");
    let config = NetworkConfig::new(None, Some(custom_dir.clone()), None);
    
    let keys_dir = config.get_keys_dir();
    assert_eq!(keys_dir, custom_dir.join("keys"));
}

#[test]
fn test_ephemeral_port() {
    // Test with ephemeral port (0)
    let config = NetworkConfig::new(Some(0), None, None);
    
    assert_eq!(config.port, 0);
    let socket_addr = config.get_socket_addr();
    match socket_addr {
        SocketAddr::V4(addr) => {
            assert_eq!(addr.port(), 0);
        },
        SocketAddr::V6(addr) => {
            assert_eq!(addr.port(), 0);
        }
    }
} 