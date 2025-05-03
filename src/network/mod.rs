use libp2p::{
    identity,
    noise,
    swarm::SwarmEvent,
    tcp,
    yamux,
    PeerId,
    SwarmBuilder,
    Multiaddr,
    mdns,
    kad,
};
use futures::StreamExt;
use std::{error::Error, time::Duration, path::Path};
use tokio::{select, sync::mpsc, fs::File, io::AsyncReadExt};
use uuid;

use crate::discovery::{self, BehaviorEvent, SwarmCommand};
use crate::protocol;
use crate::file_transfer::request_handler;
use crate::file_transfer::types::{ProtocolRequest, ProtocolResponse};

/// Strip the peer ID component from a Multiaddr if present, to avoid handshake errors
fn strip_peer_id_component(addr: Multiaddr) -> Multiaddr {
    let components: Vec<_> = addr.iter().collect();
    let mut filtered = Multiaddr::empty();
    
    for protocol in components {
        // Skip p2p components which contain the peer ID
        if !protocol.to_string().contains("p2p") {
            filtered.push(protocol);
        }
    }
    
    filtered
}

/// Generate a new peer ID and key pair
pub fn generate_peer_id() -> (PeerId, identity::Keypair) {
    let keypair = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(keypair.public());
    (peer_id, keypair)
}

/// Start a new P2P node
pub async fn start_node(port: u16) -> Result<(), Box<dyn Error>> {
    let (local_peer_id, local_key) = generate_peer_id();
    println!("🔑 Peer ID: {}", local_peer_id);

    // Clone the keypair for the behavior before it's moved to the SwarmBuilder
    let behavior_key = local_key.clone();

    // Create channel for sending commands to the swarm
    let (tx, mut rx) = mpsc::channel(32);
    
    // Initialize the swarm command channel in discovery module
    discovery::init_swarm_channel(tx.clone());

    // Build the swarm using the builder pattern with explicit transport configuration
    let mut swarm = SwarmBuilder::with_existing_identity(local_key)
        .with_tokio()
        .with_tcp(
            tcp::Config::default().nodelay(true), // Simple TCP config
            noise::Config::new, // Use noise::Config::new for authentication
            yamux::Config::default, // Use yamux::Config::default for multiplexing
        )?
        .with_behaviour(|_| discovery::create_behavior(local_peer_id.clone(), behavior_key.clone()).expect("Behavior creation failed"))?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    // Listen on all interfaces and the provided port
    let listen_addr = format!("/ip4/0.0.0.0/tcp/{}", port).parse()?;
    swarm.listen_on(listen_addr)?;

    println!("👂 Listening for events...");

    // Event loop
    loop {
        select! {
            event = swarm.select_next_some() => match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("📡 Listening on {}", address);
                }
                SwarmEvent::Behaviour(event) => {
                    println!("➡️ Node Event (Behaviour): {:?}", event); // Log all behaviour events
                    // Handle specific discovery events
                    match event {
                        BehaviorEvent::Mdns(mdns::Event::Discovered(peers)) => {
                            for (peer_id, addr) in peers {
                                handle_discovered_peer(&mut swarm, &local_peer_id, peer_id, addr);
                            }
                        }
                        BehaviorEvent::Kad(kad::Event::OutboundQueryProgressed { 
                            result: kad::QueryResult::GetClosestPeers(Ok(kad::GetClosestPeersOk { peers, .. })),
                            ..
                        }) => {
                            println!("📚 Kademlia found {} peers", peers.len());
                            // You can store these peers too if needed
                        }
                        BehaviorEvent::KeepAlive(ping) => {
                            // Handle ping results
                            if let libp2p::ping::Event { peer, result: Ok(duration), .. } = ping {
                                println!("🏓 Ping to {} is {}ms", peer, duration.as_millis());
                            }
                        }
                        // Re-enable FileTransfer handling
                        BehaviorEvent::FileTransfer(event) => {
                            // Delegate file transfer events to the protocol module
                            if let Err(e) = protocol::handle_behavior_event(&mut swarm, event).await {
                                println!("❌ Error handling file transfer event: {}", e);
                            }
                        }
                        _ => {}
                    }
                }
                SwarmEvent::ConnectionEstablished { peer_id, endpoint, num_established, .. } => {
                    println!("➡️ Node Event (ConnectionEstablished): Peer: {}, Endpoint: {:?}, NumEstablished: {}", 
                             peer_id, endpoint.get_remote_address(), num_established);
                    if peer_id == local_peer_id {
                        println!("🤝 Connection established with: {}", peer_id);
                        // Store the connected peer address
                        let addr = endpoint.get_remote_address();
                        
                        // Extract the base address without the p2p component
                        let base_addr = addr.clone().into_iter()
                            .filter(|p| !p.to_string().contains("p2p"))
                            .collect::<Multiaddr>();
                        
                        discovery::add_discovered_peer(peer_id.clone(), vec![base_addr]);
                        
                        // Mark peer as connected
                        discovery::mark_peer_connected(peer_id);
                    }
                }
                SwarmEvent::ConnectionClosed { peer_id, cause, num_established, .. } => {
                    println!("➡️ Node Event (ConnectionClosed): Peer: {}, Cause: {:?}, Remaining: {}", 
                             peer_id, cause, num_established);
                    
                    // Mark peer as disconnected
                    discovery::mark_peer_disconnected(&peer_id);
                }
                SwarmEvent::OutgoingConnectionError { peer_id, ref error, .. } => {
                    println!("➡️ OutgoingConnectionError event for Peer: {:?}, Error: {}", peer_id, error);
                    if peer_id.as_ref() == Some(&local_peer_id) { 
                        println!("❌ Failed to dial self? {:?}: {}", peer_id, error);
                    } else {
                        println!("❌ Failed to dial {:?}: {}", peer_id, error);
                    }
                }
                SwarmEvent::IncomingConnection { connection_id, local_addr, send_back_addr } => {
                    println!("➡️ Node Event (IncomingConnection): ID: {:?}, Local: {}, Remote: {}", 
                             connection_id, local_addr, send_back_addr);
                }
                SwarmEvent::IncomingConnectionError { connection_id, local_addr, send_back_addr, error } => {
                    println!("➡️ Node Event (IncomingConnectionError): ID: {:?}, Local: {}, Remote: {}, Error: {}", 
                             connection_id, local_addr, send_back_addr, error);
                }
                // Handle other swarm events as needed
                other_event => {
                     println!("➡️ Node Event (Other): {:?}", other_event);
                }
            },
            
            // Handle incoming commands from other parts of the application
            Some(cmd) = rx.recv() => match cmd {
                SwarmCommand::DialPeer(peer_id) => {
                    if !should_dial_peer(&local_peer_id, &peer_id) {
                        println!("⚠️ Skipping dial to peer {}", peer_id);
                        continue;
                    }
                    
                    if let Some(addr) = discovery::get_peer_addresses(&peer_id)
                        .and_then(|addrs| addrs.into_iter().next()) {
                        
                        println!("Dialing peer: {}", peer_id);
                        // Strip the p2p component if present to avoid the "Handshake failed" error
                        let dial_addr = strip_peer_id_component(addr.clone());
                        println!("Using address: {}", dial_addr);
                        
                        // Create a multiaddr without peer_id component for dialing
                        match swarm.dial(dial_addr) {
                            Ok(_) => println!("✅ Dialing initiated to {}", peer_id),
                            Err(e) => println!("❌ Failed to dial {}: {}", peer_id, e),
                        }
                    } else {
                        println!("❌ No addresses known for peer: {}", peer_id);
                    }
                },
                SwarmCommand::SendFile { peer_id, file_path, encrypt: _ } => {
                    println!("📤 Preparing to send file: {} to {}", file_path, peer_id);
                    
                    // Check if the file exists
                    match tokio::fs::metadata(&file_path).await {
                        Ok(metadata) => {
                            let file_size = metadata.len();
                            let file_name = std::path::Path::new(&file_path)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("unknown")
                                .to_string();
                            
                            println!("📤 Sending file: {} ({} bytes) to {}", file_name, file_size, peer_id);
                            
                            // Generate a transfer ID
                            let transfer_id = uuid::Uuid::new_v4().to_string();
                            
                            // Create a file transfer manager if we don't already have one
                            let _file_transfer_manager = crate::file_transfer::FileTransferManager::new("downloads".to_string());
                            
                            // For now, just log that we would be sending the file
                            // In a real implementation, you'd initialize the manager and send the first message
                            println!("✅ File transfer initiated with ID: {}", transfer_id);
                            println!("   (Actual file transfer not yet implemented in this version)");
                        },
                        Err(e) => {
                            println!("❌ Failed to access file {}: {}", file_path, e);
                        }
                    }
                }
            }
        }
    }
}

// Handle mDNS discovered peers
fn handle_discovered_peer(
    swarm: &mut libp2p::Swarm<discovery::Behavior>,
    local_peer_id: &PeerId,
    peer_id: PeerId, 
    addr: Multiaddr
) {
    // Skip if we discovered ourselves
    if &peer_id == local_peer_id {
        return;
    }
    
    println!("🔍 mDNS discovered peer: {} at {}", peer_id, addr);
    
    // Extract the base address without the p2p component
    let base_addr = strip_peer_id_component(addr.clone());
    
    // Store the peer in our list with the base address
    discovery::add_discovered_peer(peer_id.clone(), vec![base_addr.clone()]);
    
    // Re-enable adding to Kademlia immediately upon discovery
    swarm.behaviour_mut().kad.add_address(&peer_id, base_addr);
    println!("📚 Added peer to Kademlia: {}", peer_id);
}

// Determine if we should dial a peer
fn should_dial_peer(local_peer_id: &PeerId, peer_id: &PeerId) -> bool {
    // Don't dial ourselves
    if local_peer_id == peer_id {
        return false;
    }
    
    // Don't dial already connected peers
    if discovery::is_peer_connected(peer_id) {
        return false;
    }
    
    true
}

/// Start a temporary node just for sending a file
pub async fn start_temp_node_and_send_file(
    target_peer_id: PeerId,
    file_path: String,
    encrypt: bool,
) -> Result<(), Box<dyn Error>> {
    let (local_peer_id, local_key) = generate_peer_id();
    println!("🔑 Temporary node Peer ID: {}", local_peer_id);

    // Create network behavior
    let behavior = discovery::create_behavior(local_peer_id.clone(), local_key.clone())?;

    // Create channel for sending commands to the swarm (although we might not need it here)
    let (tx, _rx) = mpsc::channel::<SwarmCommand>(32); // Use SwarmCommand type explicitly
    discovery::init_swarm_channel(tx);

    // Build the swarm
    let mut swarm = SwarmBuilder::with_existing_identity(local_key)
        .with_tokio()
        .with_tcp(
            tcp::Config::default().nodelay(true),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_behaviour(|_| behavior)?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    // Listen on a random port
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    println!("👂 Temporary node listening for events...");
    println!("🔍 Looking for target peer: {}", target_peer_id);

    // Set a timeout for the entire operation
    let operation_timeout = tokio::time::sleep(Duration::from_secs(60)); // Increased timeout
    tokio::pin!(operation_timeout);

    // Track our state more explicitly
    let mut connection_dialed = false;
    let mut connection_established = false;
    let mut request_sent = false;
    let mut transfer_response_received = false;
    let mut transfer_accepted = false;
    let mut transfer_id: Option<String> = None; // Track the UUID transfer ID
    let mut final_response_received = false;

    // const CHUNK_SIZE: usize = 64 * 1024; // Not needed

    // Event loop with timeout
    loop {
        select! {
            _ = &mut operation_timeout => {
                println!("⏰ Operation timeout reached. Aborting file transfer.");
                if !transfer_response_received {
                    println!("❌ Never received a response from the peer.");
                } else if !transfer_accepted {
                    println!("❌ Transfer was rejected or failed.");
                }
                break;
            }
            event = swarm.select_next_some() => {
                println!("➡️  Temp Node Event: {:?}", event); // Add general event logging
                match event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        println!("📡 Temporary node listening on {}", address);
                    }
                    SwarmEvent::Behaviour(event) => {
                        match event {
                            BehaviorEvent::Mdns(mdns::Event::Discovered(peers)) => {
                                for (peer_id, addr) in peers {
                                    if peer_id == target_peer_id && !connection_dialed {
                                        println!("🎯 Found target peer {} at {}", peer_id, addr);
                                        let base_addr = strip_peer_id_component(addr.clone());
                                        
                                        println!("📚 Added target peer {} to Kademlia routing table", peer_id);
                                        
                                        println!(" Attempting to dial target {} at {}", peer_id, base_addr);
                                        /* // Don't dial immediately, let the swarm handle it?
                                        match swarm.dial(base_addr.clone()) {
                                            Ok(_) => {
                                                println!("✅ Dialing initiated to target {}", peer_id);
                                                connection_dialed = true;
                                            }
                                            Err(e) => println!("❌ Failed to dial target {}: {}", peer_id, e),
                                        } */
                                    }
                                }
                            }
                            // Re-enable FileTransfer event handling
                            BehaviorEvent::FileTransfer(request_handler::Event::Message {
                                peer, message: request_handler::Message::Response { request_id, response }, ..
                            }) => {
                                if peer == target_peer_id {
                                    println!("📤 Received file transfer response: {:?}, Request ID: {:?}", response, request_id); // Log req id for info
                                    transfer_response_received = true; // Mark that *some* response came
                                    
                                    // Handle response directly based on peer match
                                    match response {
                                        ProtocolResponse::FileReceived { success, .. } => {
                                            println!("➡️ Received FileReceived confirmation. Success: {}", success);
                                            final_response_received = true; // Treat this as the final response
                                        }
                                    }
                                }
                            }
                            BehaviorEvent::FileTransfer(request_handler::Event::OutboundFailure { peer, request_id, error, .. }) => {
                                if peer == target_peer_id {
                                    println!("❌ Outbound file transfer interaction failed to {}: {:?}, Request ID was: {:?}", 
                                        peer, error, request_id); // Log the request_id for info
                                    // We might not know if this was the initial request or a chunk, so 
                                    // potentially abort the transfer or set an error state.
                                    // For now, we'll let the main timeout handle it if it was critical.
                                }
                            }
                            _ => {}
                        }
                    }
                    SwarmEvent::ConnectionEstablished { peer_id, endpoint, num_established, .. } => {
                        println!("➡️ ConnectionEstablished event for Peer: {}, Endpoint: {:?}, NumEstablished: {}", 
                                 peer_id, endpoint.get_remote_address(), num_established);
                        if peer_id == target_peer_id {
                            println!("✅🤝 Connection established with TARGET peer: {}", peer_id);
                            connection_established = true;
                            // Only send the request if the connection is established and not already sent
                            if !request_sent {
                                println!("📤 Initiating file transfer request for {}", file_path);
                                match protocol::send_file_to_peer(&mut swarm, &target_peer_id, &file_path, encrypt).await {
                                    Ok(req_id) => { // Expect only OutboundRequestId now
                                        println!("✅ File transfer request sent, ID: {:?}", req_id);
                                        request_sent = true;
                                        // transfer_id = Some(generated_transfer_id); // No string ID returned
                                    },
                                    Err(e) => {
                                        println!("❌ Failed to send file transfer request: {}", e);
                                        // Consider breaking or retrying?
                                        break; 
                                    }
                                }
                            }
                        }
                    }
                    SwarmEvent::OutgoingConnectionError { peer_id, ref error, .. } => {
                        println!("➡️ OutgoingConnectionError event for Peer: {:?}, Error: {}", peer_id, error);
                        if Some(target_peer_id) == peer_id {
                            println!("❌ Specific failure connecting to TARGET peer {}: {}", target_peer_id, error);
                            // Log specific DialFailure reasons if available
                            if let libp2p::swarm::DialError::Transport(errors) = error {
                                for (addr, error) in errors {
                                     println!("    -> Transport error for addr {}: {}", addr, error);
                                }
                            }
                            // Assign a placeholder UUID string to transfer_id if None
                            if transfer_id.is_none() {
                                transfer_id = Some(uuid::Uuid::new_v4().to_string());
                            }
                            connection_dialed = false; // Allow retrying dial if discovered again
                        }
                    }
                    SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                        println!("➡️ ConnectionClosed event for Peer: {}, Cause: {:?}", peer_id, cause);
                        if peer_id == target_peer_id {
                             println!("👋 Connection closed with TARGET peer: {}, cause: {:?}", peer_id, cause);
                             // Reset state if needed, maybe break
                             break;
                        }
                    }
                    _ => {}
                }
            }
        }
        
        // Exit conditions
        if final_response_received { // Exit only when FileReceived comes back
            if transfer_accepted { // transfer_accepted is now potentially stale, maybe remove?
                println!("✅ File transfer process finished (received final response).");
            } else {
                println!("❌ File transfer rejected or failed (based on initial response or EOF sent without acceptance).");
            }
            break; // Exit loop once we have a definitive response or failure
        }
    }
    
    println!("🚪 Temporary node shutting down.");
    Ok(())
} 