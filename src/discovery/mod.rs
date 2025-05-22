use libp2p::{
    identity,
    kad,
    mdns,
    ping,
    swarm::NetworkBehaviour,
    Multiaddr,
    PeerId,
};
use std::{collections::{HashMap, HashSet}, sync::{Arc, Mutex}};
use tokio::sync::mpsc;
use once_cell::sync::Lazy;
use crate::file_transfer::request_handler;
// pub mod request_handler;

// Global state for discovered peers using Lazy for safe initialization
static DISCOVERED_PEERS: Lazy<Arc<Mutex<HashMap<PeerId, Vec<Multiaddr>>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

// Global channel for swarm commands
static SWARM_CHANNEL: Lazy<Arc<Mutex<Option<mpsc::Sender<SwarmCommand>>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(None)));

// Track connected peers
static CONNECTED_PEERS: Lazy<Arc<Mutex<HashSet<PeerId>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(HashSet::new())));

/// Commands that can be sent to the swarm
#[derive(Debug)]
pub enum SwarmCommand {
    DialPeer(PeerId),
    SendFile {
        peer_id: PeerId,
        file_path: String,
    },
}

/// Initialize the global channel for sending commands to the swarm
pub fn init_swarm_channel(channel: mpsc::Sender<SwarmCommand>) {
    let mut tx = SWARM_CHANNEL.lock().unwrap();
    *tx = Some(channel);
}

/// Get the swarm channel for sending commands
pub fn get_swarm_channel() -> Option<mpsc::Sender<SwarmCommand>> {
    SWARM_CHANNEL.lock().unwrap().clone()
}

/// Custom network behavior for the node
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "BehaviorEvent")]
pub struct Behavior {
    pub kad: kad::Behaviour<kad::store::MemoryStore>,
    pub mdns: mdns::tokio::Behaviour,
    pub ping: ping::Behaviour,
    pub handshake: request_handler::Behaviour<request_handler::FileTransferCodec>,
}

/// Events emitted by the network behavior
#[derive(Debug)]
pub enum BehaviorEvent {
    Mdns(mdns::Event),
    Kad(kad::Event),
    KeepAlive(ping::Event),
    Handshake(request_handler::Event<crate::file_transfer::types::ProtocolRequest, crate::file_transfer::types::ProtocolResponse>),
}

impl From<mdns::Event> for BehaviorEvent {
    fn from(event: mdns::Event) -> Self {
        BehaviorEvent::Mdns(event)
    }
}

impl From<kad::Event> for BehaviorEvent {
    fn from(event: kad::Event) -> Self {
        BehaviorEvent::Kad(event)
    }
}

impl From<ping::Event> for BehaviorEvent {
    fn from(event: ping::Event) -> Self {
        BehaviorEvent::KeepAlive(event)
    }
}

impl From<request_handler::Event<crate::file_transfer::types::ProtocolRequest, crate::file_transfer::types::ProtocolResponse>> for BehaviorEvent {
    fn from(event: request_handler::Event<crate::file_transfer::types::ProtocolRequest, crate::file_transfer::types::ProtocolResponse>) -> Self {
        BehaviorEvent::Handshake(event)
    }
}

/// Create a new network behavior
pub fn create_behavior(
    local_peer_id: PeerId,
    _local_key: identity::Keypair,
) -> Result<Behavior, Box<dyn std::error::Error>> {
    // use libp2p::stream; // Remove this line

    // Create Kademlia behavior for DHT
    let store = kad::store::MemoryStore::new(local_peer_id.clone());
    
    // Create Kademlia with the store
    let kad_behavior = kad::Behaviour::new(local_peer_id.clone(), store);

    // Create mDNS behavior for local discovery
    let mdns_behavior = mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id.clone())?;

    // Create ping behavior for keeping connections alive
    let ping_behavior = ping::Behaviour::new(ping::Config::new());

    // Create file transfer request-response behavior
    let file_transfer_behavior = request_handler::create_request_response();

    Ok(Behavior {
        kad: kad_behavior,
        mdns: mdns_behavior,
        ping: ping_behavior,
        handshake: file_transfer_behavior,
        // stream: stream::Behaviour::new(),  // Remove stream behavior initialization
    })
}

/// Add a discovered peer to the global state
pub fn add_discovered_peer(peer_id: PeerId, addresses: Vec<Multiaddr>) {
    let mut peers_map = DISCOVERED_PEERS.lock().unwrap();
    
    // Update existing addresses or add new peer
    if let Some(addrs) = peers_map.get_mut(&peer_id) {
        // Add new addresses that aren't already in the list
        for addr in addresses {
            if !addrs.contains(&addr) {
                addrs.push(addr);
            }
        }
    } else {
        // Add new peer with its addresses
        peers_map.insert(peer_id, addresses);
    }
}

/// Get addresses for a specific peer
pub fn get_peer_addresses(peer_id: &PeerId) -> Option<Vec<Multiaddr>> {
    let peers_map = DISCOVERED_PEERS.lock().unwrap();
    peers_map.get(peer_id).cloned()
}

/// Get all discovered peers
pub fn get_discovered_peers() -> HashMap<PeerId, Vec<Multiaddr>> {
    let peers_map = DISCOVERED_PEERS.lock().unwrap();
    peers_map.clone()
}

/// Dial a peer using its ID
pub fn dial_peer(peer_id: PeerId) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(tx) = get_swarm_channel() {
        let _ = tx.try_send(SwarmCommand::DialPeer(peer_id));
        Ok(())
    } else {
        Err("Swarm channel not initialized".into())
    }
}

/// Send a file to a peer
pub fn send_file(peer_id: PeerId, file_path: String) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(tx) = get_swarm_channel() {
        let _ = tx.try_send(SwarmCommand::SendFile {
            peer_id,
            file_path,
        });
        Ok(())
    } else {
        Err("Swarm channel not initialized".into())
    }
}

/// Mark a peer as connected
pub fn mark_peer_connected(peer_id: PeerId) {
    let mut connected = CONNECTED_PEERS.lock().unwrap();
    connected.insert(peer_id);
}

/// Mark a peer as disconnected
pub fn mark_peer_disconnected(peer_id: &PeerId) {
    let mut connected = CONNECTED_PEERS.lock().unwrap();
    connected.remove(peer_id);
}

/// Check if a peer is already connected
pub fn is_peer_connected(peer_id: &PeerId) -> bool {
    let connected = CONNECTED_PEERS.lock().unwrap();
    connected.contains(peer_id)
}

/// Build a swarm with the network behavior
pub fn build_swarm<T>(
    local_peer_id: PeerId,
    local_key: identity::Keypair,
    _transport: T,
) -> Result<libp2p::swarm::Swarm<Behavior>, Box<dyn std::error::Error>>
where
    T: libp2p::core::transport::Transport + 'static,
    T::Output: libp2p::core::muxing::StreamMuxer + Send + Sync + 'static,
    <T::Output as libp2p::core::muxing::StreamMuxer>::Substream: futures::AsyncRead + futures::AsyncWrite + Unpin,
{
    // Create the behavior
    let behavior = create_behavior(local_peer_id, local_key.clone())?;
    
    // Build the swarm
    let swarm = libp2p::SwarmBuilder::with_existing_identity(local_key)
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )?
        .with_behaviour(|_| Ok(behavior))?
        .with_swarm_config(|_| libp2p::swarm::Config::with_tokio_executor())
        .build();
    
    Ok(swarm)
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::{identity, PeerId};

    #[tokio::test]
    async fn test_discovery_behavior_creation() {
        // Create a test identity
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        
        // Create discovery behavior - just test that it completes without error
        let discovery = create_behavior(local_peer_id, local_key.clone());
        assert!(discovery.is_ok(), "Should create behavior without errors");
    }
}