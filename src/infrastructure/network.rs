use crate::core::{
    domain::{DomainEvent, PeerId as DomainPeerId},
    traits::{DomainResult, EventPublisher, NetworkService},
};
use crate::file_transfer::{
    FileTransferCodec, FileTransferProtocol, ProtocolRequest, ProtocolResponse,
};
use crate::infrastructure::config::AppConfig;
use async_trait::async_trait;
use futures::stream::StreamExt;
use libp2p::{
    Multiaddr, PeerId, Swarm, SwarmBuilder, gossipsub, identify, identity, kad, mdns, noise,
    request_response,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, error, info, warn};

/// Network behavior combining all libp2p protocols including advanced features
#[derive(NetworkBehaviour)]
pub struct CipherStreamBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub identify: identify::Behaviour,
    pub request_response: request_response::Behaviour<FileTransferCodec>,
    pub mdns: mdns::tokio::Behaviour,
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
}

/// Network events internal to the service
#[derive(Debug)]
pub enum NetworkEvent {
    PeerConnected(PeerId),
    PeerDisconnected(PeerId),
    FileTransferRequest {
        from: PeerId,
        request: ProtocolRequest,
    },
    FileTransferResponse {
        from: PeerId,
        response: ProtocolResponse,
    },
    GossipMessage {
        from: PeerId,
        topic: String,
        data: Vec<u8>,
    },
}

/// Commands that can be sent to the network service
#[derive(Debug)]
pub enum NetworkCommand {
    StartListening(u16),
    ConnectToPeer(Multiaddr),
    SendFileRequest {
        peer_id: PeerId,
        request: ProtocolRequest,
    },
    SubscribeTopic(String),
    PublishMessage {
        topic: String,
        data: Vec<u8>,
    },
    // Advanced peer discovery commands
    StartMdnsDiscovery,
    StopMdnsDiscovery,
    BootstrapKademlia(Vec<Multiaddr>),
    FindClosestPeers(PeerId),
    AddKademliaAddress {
        peer_id: PeerId,
        addr: Multiaddr,
    },
}

/// Network service implementation using libp2p 0.55
pub struct LibP2pNetworkService {
    command_tx: mpsc::UnboundedSender<NetworkCommand>,
    #[allow(dead_code)] // Part of future API for receiving network events
    event_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<NetworkEvent>>>,
    local_peer_id: PeerId,
}

impl LibP2pNetworkService {
    /// Create a new libp2p network service
    pub async fn new(
        _config: Arc<AppConfig>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> DomainResult<Self> {
        // Generate or load keypair
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        info!("Local peer id: {}", local_peer_id);

        // Configure gossipsub
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .message_id_fn(message_id_fn)
            .build()
            .map_err(|e| format!("Failed to build gossipsub config: {}", e))?;

        let gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(local_key.clone()),
            gossipsub_config,
        )
        .map_err(|e| format!("Failed to create gossipsub: {}", e))?;

        // Configure identify
        let identify = identify::Behaviour::new(identify::Config::new(
            "/cipherstream/1.0.0".to_string(),
            local_key.public(),
        ));

        // Configure request-response for file transfers
        let protocols = [(
            FileTransferProtocol::new(),
            request_response::ProtocolSupport::Full,
        )];
        let request_response = request_response::Behaviour::with_codec(
            FileTransferCodec,
            protocols,
            request_response::Config::default(),
        );

        // Configure mDNS for local peer discovery
        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)
            .map_err(|e| format!("Failed to create mDNS: {}", e))?;

        // Configure Kademlia DHT for global peer routing
        let mut kademlia =
            kad::Behaviour::new(local_peer_id, kad::store::MemoryStore::new(local_peer_id));

        // Set Kademlia to server mode to respond to DHT queries
        kademlia.set_mode(Some(kad::Mode::Server));

        // Add well-known IPFS bootstrap peers for global DHT connectivity
        let bootstrap_peers = vec![
            // IPFS bootstrap nodes
            "/dnsaddr/bootstrap.libp2p.io/p2p/QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN",
            "/dnsaddr/bootstrap.libp2p.io/p2p/QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa",
            "/dnsaddr/bootstrap.libp2p.io/p2p/QmbLHAnMoJPWSCR5Zp7y9BDkkFBhYZyEjhY5bGHxpmmk9N",
            "/ip4/104.131.131.82/tcp/4001/p2p/QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ",
        ];

        for addr_str in &bootstrap_peers {
            let Ok(addr) = addr_str.parse::<Multiaddr>() else { continue };
            let Some(peer_id) = addr.iter().find_map(|p| {
                if let libp2p::multiaddr::Protocol::P2p(peer_id) = p {
                    Some(peer_id)
                } else {
                    None
                }
            }) else { continue };
            kademlia.add_address(&peer_id, addr);
            info!("Added Kademlia bootstrap peer: {}", peer_id);
        }

        // Create behaviour
        let behaviour = CipherStreamBehaviour {
            gossipsub,
            identify,
            request_response,
            mdns,
            kademlia,
        };

        // Build swarm using the new libp2p 0.55 API
        let swarm = SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )
            .map_err(|e| format!("Failed to build transport: {}", e))?
            .with_behaviour(|_| Ok(behaviour))
            .map_err(|e| format!("Failed to build behaviour: {}", e))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(30)))
            .build();

        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        // Spawn the swarm task
        tokio::spawn(Self::run_swarm_task(
            swarm,
            command_rx,
            event_tx,
            event_publisher,
        ));

        Ok(Self {
            command_tx,
            event_rx: Arc::new(tokio::sync::Mutex::new(event_rx)),
            local_peer_id,
        })
    }

    /// Start the network service
    pub async fn start(&self, port: u16) -> DomainResult<()> {
        self.command_tx
            .send(NetworkCommand::StartListening(port))
            .map_err(|e| format!("Failed to send start command: {}", e))?;
        Ok(())
    }

    /// Main swarm task that handles all swarm operations
    async fn run_swarm_task(
        mut swarm: Swarm<CipherStreamBehaviour>,
        mut command_rx: mpsc::UnboundedReceiver<NetworkCommand>,
        event_tx: mpsc::UnboundedSender<NetworkEvent>,
        event_publisher: Arc<dyn EventPublisher>,
    ) {
        let mut connected_peers: HashMap<PeerId, Vec<Multiaddr>> = HashMap::new();
        let mut bootstrap_attempted = false;

        loop {
            tokio::select! {
                // Handle commands from the service
                Some(command) = command_rx.recv() => {
                    if let Err(e) = Self::handle_command(&mut swarm, command).await {
                        error!("Error handling command: {}", e);
                    }
                }

                // Handle swarm events
                event = swarm.select_next_some() => {
                    // Trigger Kademlia bootstrap once we start listening
                    if !bootstrap_attempted && matches!(event, SwarmEvent::NewListenAddr { .. }) {
                        if let Err(e) = swarm.behaviour_mut().kademlia.bootstrap() {
                            warn!("Kademlia bootstrap failed: {:?}", e);
                        } else {
                            info!("Kademlia bootstrap initiated successfully");
                            bootstrap_attempted = true;
                        }
                    }

                    if let Err(e) = Self::handle_swarm_event(
                        event,
                        &event_tx,
                        &event_publisher,
                        &mut connected_peers,
                    ).await {
                        error!("Error handling swarm event: {}", e);
                    }
                }
            }
        }
    }

    /// Handle commands sent to the swarm
    async fn handle_command(
        swarm: &mut Swarm<CipherStreamBehaviour>,
        command: NetworkCommand,
    ) -> DomainResult<()> {
        match command {
            NetworkCommand::StartListening(port) => {
                let listen_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", port)
                    .parse()
                    .map_err(|e| format!("Invalid listen address: {}", e))?;

                swarm
                    .listen_on(listen_addr.clone())
                    .map_err(|e| format!("Failed to start listening: {}", e))?;

                info!("Network service started on {}", listen_addr);
            }
            NetworkCommand::ConnectToPeer(addr) => {
                swarm
                    .dial(addr.clone())
                    .map_err(|e| format!("Failed to dial {}: {}", addr, e))?;
            }
            NetworkCommand::SendFileRequest { peer_id, request } => {
                let _request_id = swarm
                    .behaviour_mut()
                    .request_response
                    .send_request(&peer_id, request);
                info!("Sent file transfer request to {}", peer_id);
            }
            NetworkCommand::SubscribeTopic(topic) => {
                let topic = gossipsub::IdentTopic::new(topic);
                swarm
                    .behaviour_mut()
                    .gossipsub
                    .subscribe(&topic)
                    .map_err(|e| format!("Failed to subscribe to topic: {}", e))?;
                info!("Subscribed to topic: {}", topic);
            }
            NetworkCommand::PublishMessage { topic, data } => {
                let topic = gossipsub::IdentTopic::new(topic);
                let _message_id = swarm
                    .behaviour_mut()
                    .gossipsub
                    .publish(topic.clone(), data)
                    .map_err(|e| format!("Failed to publish message: {}", e))?;
                info!("Published message to topic: {}", topic);
            }
            NetworkCommand::StartMdnsDiscovery => {
                info!("mDNS discovery is automatically enabled");
            }
            NetworkCommand::StopMdnsDiscovery => {
                info!("mDNS discovery is automatically managed");
            }
            NetworkCommand::BootstrapKademlia(peers) => {
                if let Err(e) = swarm.behaviour_mut().kademlia.bootstrap() {
                    warn!("Kademlia bootstrap failed: {:?}", e);
                } else {
                    info!("Kademlia bootstrap started");
                }

                // Add bootstrap peers to routing table
                for addr in peers {
                    if let Some(peer_id) = addr.iter().find_map(|p| {
                        if let libp2p::multiaddr::Protocol::P2p(peer_id) = p {
                            Some(peer_id)
                        } else {
                            None
                        }
                    }) {
                        swarm
                            .behaviour_mut()
                            .kademlia
                            .add_address(&peer_id, addr.clone());
                    }
                }
            }
            NetworkCommand::FindClosestPeers(peer_id) => {
                let _ = swarm.behaviour_mut().kademlia.get_closest_peers(peer_id);
                debug!("Kademlia finding closest peers to {}", peer_id);
            }
            NetworkCommand::AddKademliaAddress { peer_id, addr } => {
                let addr_clone = addr.clone();
                swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
                info!("Kademlia added address {} to peer {}", addr_clone, peer_id);
            }
        }
        Ok(())
    }

    /// Handle individual swarm events
    async fn handle_swarm_event(
        event: SwarmEvent<CipherStreamBehaviourEvent>,
        event_tx: &mpsc::UnboundedSender<NetworkEvent>,
        event_publisher: &Arc<dyn EventPublisher>,
        connected_peers: &mut HashMap<PeerId, Vec<Multiaddr>>,
    ) -> DomainResult<()> {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("Listening on {}", address);
            }
            SwarmEvent::ConnectionEstablished {
                peer_id, endpoint, ..
            } => {
                info!("Connected to peer: {}", peer_id);

                // Store peer address
                connected_peers
                    .entry(peer_id)
                    .or_default()
                    .push(endpoint.get_remote_address().clone());

                // Send internal event
                let _ = event_tx.send(NetworkEvent::PeerConnected(peer_id));

                // Publish domain event
                let domain_peer_id = DomainPeerId::new(peer_id.to_string());
                let domain_event = DomainEvent::PeerConnected {
                    peer_id: domain_peer_id,
                };
                let _ = event_publisher.publish(domain_event).await;
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                info!("Disconnected from peer: {}", peer_id);

                // Remove peer
                connected_peers.remove(&peer_id);

                // Send internal event
                let _ = event_tx.send(NetworkEvent::PeerDisconnected(peer_id));

                // Publish domain event
                let domain_peer_id = DomainPeerId::new(peer_id.to_string());
                let domain_event = DomainEvent::PeerDisconnected {
                    peer_id: domain_peer_id,
                };
                let _ = event_publisher.publish(domain_event).await;
            }
            SwarmEvent::Behaviour(CipherStreamBehaviourEvent::RequestResponse(event)) => {
                Self::handle_request_response_event(event, event_tx).await?;
            }
            SwarmEvent::Behaviour(CipherStreamBehaviourEvent::Gossipsub(event)) => {
                Self::handle_gossipsub_event(event, event_tx).await?;
            }
            SwarmEvent::Behaviour(CipherStreamBehaviourEvent::Identify(event)) => {
                Self::handle_identify_event(event).await?;
            }
            SwarmEvent::Behaviour(CipherStreamBehaviourEvent::Mdns(event)) => {
                Self::handle_mdns_event(event, event_tx, event_publisher, connected_peers).await?;
            }
            SwarmEvent::Behaviour(CipherStreamBehaviourEvent::Kademlia(event)) => {
                Self::handle_kademlia_event(event, event_tx, event_publisher).await?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle request-response events (file transfers)
    async fn handle_request_response_event(
        event: request_response::Event<ProtocolRequest, ProtocolResponse>,
        event_tx: &mpsc::UnboundedSender<NetworkEvent>,
    ) -> DomainResult<()> {
        match event {
            request_response::Event::Message { peer, message, .. } => match message {
                request_response::Message::Request { request, .. } => {
                    println!("📥 Received file transfer request from {}", peer);
                    let _ = event_tx.send(NetworkEvent::FileTransferRequest {
                        from: peer,
                        request,
                    });
                }
                request_response::Message::Response { response, .. } => {
                    info!("Received file transfer response from {}", peer);
                    let _ = event_tx.send(NetworkEvent::FileTransferResponse {
                        from: peer,
                        response,
                    });
                }
            },
            request_response::Event::OutboundFailure { peer, error, .. } => {
                warn!("Outbound failure to {}: {:?}", peer, error);
            }
            request_response::Event::InboundFailure { peer, error, .. } => {
                warn!("Inbound failure from {}: {:?}", peer, error);
            }
            request_response::Event::ResponseSent { .. } => {
                debug!("Response sent");
            }
        }
        Ok(())
    }

    /// Handle gossipsub events (peer discovery and messaging)
    async fn handle_gossipsub_event(
        event: gossipsub::Event,
        event_tx: &mpsc::UnboundedSender<NetworkEvent>,
    ) -> DomainResult<()> {
        match event {
            gossipsub::Event::Message {
                propagation_source: source,
                message,
                ..
            } => {
                let topic = message.topic.as_str().to_string();
                let _ = event_tx.send(NetworkEvent::GossipMessage {
                    from: source,
                    topic,
                    data: message.data,
                });
            }
            gossipsub::Event::Subscribed { peer_id, topic } => {
                debug!("Peer {} subscribed to topic: {}", peer_id, topic);
            }
            gossipsub::Event::Unsubscribed { peer_id, topic } => {
                debug!("Peer {} unsubscribed from topic: {}", peer_id, topic);
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle identify events
    async fn handle_identify_event(event: identify::Event) -> DomainResult<()> {
        match event {
            identify::Event::Received { peer_id, info, .. } => {
                debug!("Identified peer {}: {}", peer_id, info.protocol_version);
            }
            identify::Event::Sent { peer_id, .. } => {
                debug!("Sent identify info to {}", peer_id);
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle mDNS events
    async fn handle_mdns_event(
        event: mdns::Event,
        event_tx: &mpsc::UnboundedSender<NetworkEvent>,
        event_publisher: &Arc<dyn EventPublisher>,
        connected_peers: &mut HashMap<PeerId, Vec<Multiaddr>>,
    ) -> DomainResult<()> {
        match event {
            mdns::Event::Discovered(list) => {
                for (peer_id, _) in list {
                    info!("mDNS discovered peer: {}", peer_id);

                    // Send internal event
                    let _ = event_tx.send(NetworkEvent::PeerConnected(peer_id));

                    // Publish domain event
                    let domain_peer_id = DomainPeerId::new(peer_id.to_string());
                    let domain_event = DomainEvent::PeerConnected {
                        peer_id: domain_peer_id,
                    };
                    let _ = event_publisher.publish(domain_event).await;
                }
            }
            mdns::Event::Expired(list) => {
                for (peer_id, _) in list {
                    info!("mDNS peer expired: {}", peer_id);

                    // Remove peer
                    connected_peers.remove(&peer_id);

                    // Send internal event
                    let _ = event_tx.send(NetworkEvent::PeerDisconnected(peer_id));

                    // Publish domain event
                    let domain_peer_id = DomainPeerId::new(peer_id.to_string());
                    let domain_event = DomainEvent::PeerDisconnected {
                        peer_id: domain_peer_id,
                    };
                    let _ = event_publisher.publish(domain_event).await;
                }
            }
        }
        Ok(())
    }

    /// Handle Kademlia events
    async fn handle_kademlia_event(
        event: kad::Event,
        event_tx: &mpsc::UnboundedSender<NetworkEvent>,
        event_publisher: &Arc<dyn EventPublisher>,
    ) -> DomainResult<()> {
        match event {
            kad::Event::OutboundQueryProgressed { result, .. } => {
                match result {
                    kad::QueryResult::GetClosestPeers(Ok(kad::GetClosestPeersOk {
                        peers, ..
                    })) => {
                        debug!("Kademlia found {} close peers", peers.len());
                        for peer_info in peers {
                            let _ = event_tx.send(NetworkEvent::PeerConnected(peer_info.peer_id));
                        }
                    }
                    kad::QueryResult::Bootstrap(Ok(kad::BootstrapOk { num_remaining, .. })) => {
                        if num_remaining == 0 {
                            info!("Kademlia bootstrap complete - connected to DHT network");
                        } else {
                            debug!(
                                "Kademlia bootstrap in progress... {} queries remaining",
                                num_remaining
                            );
                        }
                    }
                    kad::QueryResult::Bootstrap(Err(e)) => {
                        warn!("Kademlia bootstrap failed: {:?}", e);
                    }
                    _ => {} // Handle other query results as needed
                }
            }
            kad::Event::RoutingUpdated { peer, .. } => {
                debug!("Kademlia routing table updated for peer: {}", peer);

                // Send internal event
                let _ = event_tx.send(NetworkEvent::PeerConnected(peer));

                // Publish domain event
                let domain_peer_id = DomainPeerId::new(peer.to_string());
                let domain_event = DomainEvent::PeerConnected {
                    peer_id: domain_peer_id,
                };
                let _ = event_publisher.publish(domain_event).await;
            }
            kad::Event::InboundRequest {
                request:
                    kad::InboundRequest::FindNode {
                        num_closer_peers, ..
                    },
            } => {
                debug!(
                    "Kademlia received FindNode request, returning {} peers",
                    num_closer_peers
                );
            }
            _ => {} // Handle other Kademlia events as needed
        }
        Ok(())
    }

    /// Connect to a specific peer
    pub async fn connect_to_peer(&self, addr: Multiaddr) -> DomainResult<()> {
        self.command_tx
            .send(NetworkCommand::ConnectToPeer(addr))
            .map_err(|e| format!("Failed to send connect command: {}", e))?;
        Ok(())
    }

    /// Send a file transfer request
    pub async fn send_file_request(
        &self,
        peer_id: PeerId,
        request: ProtocolRequest,
    ) -> DomainResult<()> {
        self.command_tx
            .send(NetworkCommand::SendFileRequest { peer_id, request })
            .map_err(|e| format!("Failed to send file request command: {}", e))?;
        Ok(())
    }

    /// Subscribe to a gossipsub topic
    pub async fn subscribe_topic(&self, topic: &str) -> DomainResult<()> {
        self.command_tx
            .send(NetworkCommand::SubscribeTopic(topic.to_string()))
            .map_err(|e| format!("Failed to send subscribe command: {}", e))?;
        Ok(())
    }

    /// Publish a message to a gossipsub topic
    pub async fn publish_message(&self, topic: &str, data: Vec<u8>) -> DomainResult<()> {
        self.command_tx
            .send(NetworkCommand::PublishMessage {
                topic: topic.to_string(),
                data,
            })
            .map_err(|e| format!("Failed to send publish command: {}", e))?;
        Ok(())
    }

    /// Get connected peers (simplified - would need event-based tracking in real implementation)
    pub async fn get_connected_peers(&self) -> Vec<PeerId> {
        // For now, return empty vec as we'd need to implement state tracking
        // In a real implementation, we'd track this via events
        Vec::new()
    }

    /// Get local peer ID
    pub fn local_peer_id(&self) -> PeerId {
        self.local_peer_id
    }

    // Advanced peer discovery methods

    /// Bootstrap the Kademlia DHT with known peers
    pub async fn bootstrap_kademlia(&self, bootstrap_peers: Vec<Multiaddr>) -> DomainResult<()> {
        self.command_tx
            .send(NetworkCommand::BootstrapKademlia(bootstrap_peers))
            .map_err(|e| format!("Failed to send bootstrap command: {}", e))?;
        Ok(())
    }

    /// Find closest peers to a target peer ID using Kademlia
    pub async fn find_closest_peers(&self, target: PeerId) -> DomainResult<()> {
        self.command_tx
            .send(NetworkCommand::FindClosestPeers(target))
            .map_err(|e| format!("Failed to send find peers command: {}", e))?;
        Ok(())
    }

    /// Add a peer address to the Kademlia routing table
    pub async fn add_kademlia_address(&self, peer_id: PeerId, addr: Multiaddr) -> DomainResult<()> {
        self.command_tx
            .send(NetworkCommand::AddKademliaAddress { peer_id, addr })
            .map_err(|e| format!("Failed to send add address command: {}", e))?;
        Ok(())
    }

    /// Start mDNS discovery (automatically enabled)
    pub async fn start_mdns_discovery(&self) -> DomainResult<()> {
        self.command_tx
            .send(NetworkCommand::StartMdnsDiscovery)
            .map_err(|e| format!("Failed to send mDNS start command: {}", e))?;
        Ok(())
    }

    /// Collect network events for a fixed duration and return them.
    /// This is useful for short-lived discovery flows from the CLI.
    pub async fn collect_events_for(&self, duration: Duration) -> Vec<NetworkEvent> {
        use tokio::time::{Instant, sleep};
        let deadline = Instant::now() + duration;
        let mut collected: Vec<NetworkEvent> = Vec::new();

        loop {
            {
                let mut rx = self.event_rx.lock().await;
                while let Ok(event) = rx.try_recv() {
                    collected.push(event);
                }
            }

            if Instant::now() >= deadline {
                break;
            }

            sleep(Duration::from_millis(100)).await;
        }

        collected
    }
}

#[async_trait]
impl NetworkService for LibP2pNetworkService {
    async fn start_listening(&self, port: u16) -> DomainResult<()> {
        self.start(port).await
    }

    async fn send_message(
        &self,
        peer_id: &crate::core::domain::PeerId,
        message: Vec<u8>,
    ) -> DomainResult<()> {
        let _libp2p_peer_id: PeerId = peer_id
            .id
            .parse()
            .map_err(|e| format!("Invalid peer ID: {}", e))?;

        // For now, we'll use gossipsub for general messaging
        self.publish_message("cipherstream-messages", message).await
    }

    async fn broadcast_message(&self, message: Vec<u8>) -> DomainResult<()> {
        self.publish_message("cipherstream-broadcast", message)
            .await
    }
}

// Simple implementation of NetworkService for testing/fallback
pub struct SimpleNetworkService {
    local_peer_id: String,
    connected_peers: Arc<RwLock<HashMap<String, Vec<String>>>>,
    event_publisher: Option<Arc<dyn EventPublisher>>,
}

impl SimpleNetworkService {
    pub fn new() -> Self {
        Self {
            local_peer_id: format!("peer-{}", uuid::Uuid::new_v4()),
            connected_peers: Arc::new(RwLock::new(HashMap::new())),
            event_publisher: None,
        }
    }

    pub fn with_event_publisher(mut self, event_publisher: Arc<dyn EventPublisher>) -> Self {
        self.event_publisher = Some(event_publisher);
        self
    }

    /// Get connected peers
    pub async fn get_connected_peers(&self) -> Vec<String> {
        let peers = self.connected_peers.read().await;
        peers.keys().cloned().collect()
    }

    /// Get local peer ID
    pub fn local_peer_id(&self) -> &str {
        &self.local_peer_id
    }

    /// Connect to a specific peer (mock implementation)
    pub async fn connect_to_peer(&self, addr: &str) -> DomainResult<()> {
        info!("Connecting to peer at: {}", addr);

        // Mock peer ID extraction from address
        let peer_id = format!("peer-at-{}", addr);

        {
            let mut peers = self.connected_peers.write().await;
            peers.insert(peer_id.clone(), vec![addr.to_string()]);
        }

        // Publish domain event if we have an event publisher
        if let Some(ref publisher) = self.event_publisher {
            let domain_peer_id = DomainPeerId::new(peer_id);
            let domain_event = DomainEvent::PeerConnected {
                peer_id: domain_peer_id,
            };
            let _ = publisher.publish(domain_event).await;
        }

        Ok(())
    }
}

impl Default for SimpleNetworkService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NetworkService for SimpleNetworkService {
    async fn start_listening(&self, port: u16) -> DomainResult<()> {
        info!("Simple network service listening on port {}", port);
        info!("Local peer ID: {}", self.local_peer_id);
        Ok(())
    }

    async fn send_message(
        &self,
        peer_id: &crate::core::domain::PeerId,
        _message: Vec<u8>,
    ) -> DomainResult<()> {
        debug!("Sending message to peer: {}", peer_id.as_str());
        Ok(())
    }

    async fn broadcast_message(&self, _message: Vec<u8>) -> DomainResult<()> {
        debug!("Broadcasting message");
        Ok(())
    }
}

/// Message ID function for gossipsub
fn message_id_fn(message: &gossipsub::Message) -> gossipsub::MessageId {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(&message.data);
    gossipsub::MessageId::from(hasher.finalize().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::events::InMemoryEventPublisher;

    #[tokio::test]
    async fn test_simple_network_service() {
        let service = SimpleNetworkService::new();
        assert!(service.start_listening(8000).await.is_ok());
        assert!(service.get_connected_peers().await.is_empty());
    }

    #[tokio::test]
    async fn test_simple_network_service_with_events() {
        let event_publisher = Arc::new(InMemoryEventPublisher::new());
        let service = SimpleNetworkService::new().with_event_publisher(event_publisher.clone());

        assert!(service.connect_to_peer("127.0.0.1:8001").await.is_ok());
        assert_eq!(service.get_connected_peers().await.len(), 1);

        // Check that event was published
        let events = event_publisher.get_events().await;
        assert_eq!(events.len(), 1);
    }

    #[tokio::test]
    async fn test_libp2p_network_service_creation() {
        let config = Arc::new(crate::infrastructure::config::AppConfig::default());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());

        let network_service = LibP2pNetworkService::new(config, event_publisher).await;
        assert!(network_service.is_ok());

        if let Ok(service) = network_service {
            println!(
                "LibP2P service created with peer ID: {}",
                service.local_peer_id()
            );
        }
    }

    #[tokio::test]
    async fn test_libp2p_network_service_topics() {
        let config = Arc::new(crate::infrastructure::config::AppConfig::default());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());

        if let Ok(service) = LibP2pNetworkService::new(config, event_publisher).await {
            assert!(service.subscribe_topic("test-topic").await.is_ok());
            assert!(
                service
                    .publish_message("test-topic", b"test message".to_vec())
                    .await
                    .is_ok()
            );
        }
    }
}
