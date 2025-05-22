// Export the config module
pub mod config;

use std::{
    error::Error as StdError,
    io::ErrorKind,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use futures::{AsyncRead, AsyncWrite, StreamExt, io::{AsyncReadExt as FuturesAsyncReadExt, AsyncWriteExt as FuturesAsyncWriteExt}};
use libp2p::{
    identity,
    kad::{store::MemoryStore, Behaviour as KadBehaviour, Config as KademliaConfig, Event as KademliaEvent},
    mdns, noise, ping,
    relay::{client::Behaviour as RelayClientBehaviour, client::Event as RelayClientEvent},
    request_response::{self as request_handler, Codec, Config as RequestResponseConfig, Event as RequestResponseEvent, OutboundRequestId, ProtocolSupport},
    swarm::{NetworkBehaviour, Swarm, SwarmEvent, dial_opts::DialOpts},
    tcp,
    yamux, Multiaddr, PeerId,
    identify::{Behaviour as IdentifyBehaviour, Config as IdentifyConfig, Event as IdentifyEvent},
    StreamProtocol,
};
use tokio::select;
use log::{debug, error, info, warn};
use anyhow::{anyhow, Result, Error as AnyhowError};

use crate::{
    file_transfer::types::{ProtocolRequest, ProtocolResponse},
    protocol::{FILE_TRANSFER_PROTO_ID},
};

// Add a function to generate peer id
pub fn generate_peer_id() -> (PeerId, identity::Keypair) {
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    (local_peer_id, local_key)
}

#[derive(Clone, Default)]
pub struct FileTransferCodec;

#[async_trait::async_trait]
impl Codec for FileTransferCodec {
    type Protocol = StreamProtocol;
    type Request = ProtocolRequest;
    type Response = ProtocolResponse;

    async fn read_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> Result<Self::Request, std::io::Error>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut vec = Vec::new();
        FuturesAsyncReadExt::read_to_end(io, &mut vec).await?;
        let (req, _): (Self::Request, _) = bincode::decode_from_slice(&vec, bincode::config::standard())
            .map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e))?;
        Ok(req)
    }

    async fn read_response<T>(&mut self, _: &Self::Protocol, io: &mut T) -> Result<Self::Response, std::io::Error>
    where
        T: AsyncRead + Unpin + Send,
    {
         let mut vec = Vec::new();
         FuturesAsyncReadExt::read_to_end(io, &mut vec).await?;
         let (res, _): (Self::Response, _) = bincode::decode_from_slice(&vec, bincode::config::standard())
            .map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e))?;
         Ok(res)
    }

    async fn write_request<T>(&mut self, _: &Self::Protocol, io: &mut T, req: Self::Request) -> Result<(), std::io::Error>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let vec = bincode::encode_to_vec(req, bincode::config::standard())
             .map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e))?;
        FuturesAsyncWriteExt::write_all(io, &vec).await?;
        FuturesAsyncWriteExt::close(io).await?;
        Ok(())
    }

    async fn write_response<T>(&mut self, _: &Self::Protocol, io: &mut T, res: Self::Response) -> Result<(), std::io::Error>
    where
        T: AsyncWrite + Unpin + Send,
    {
         let vec = bincode::encode_to_vec(res, bincode::config::standard())
              .map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e))?;
         FuturesAsyncWriteExt::write_all(io, &vec).await?;
         FuturesAsyncWriteExt::close(io).await?;
         Ok(())
    }
}

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "BehaviorEvent")]
pub struct Behavior {
    relay_client: RelayClientBehaviour,
    ping: ping::Behaviour,
    identify: IdentifyBehaviour,
    kademlia: KadBehaviour<MemoryStore>,
    mdns: mdns::tokio::Behaviour,
    request_response: request_handler::Behaviour<FileTransferCodec>,
}

#[derive(Debug)]
pub enum BehaviorEvent {
    RelayClient(RelayClientEvent),
    Ping(ping::Event),
    Identify(IdentifyEvent),
    Kademlia(KademliaEvent),
    Mdns(mdns::Event),
    RequestResponse(RequestResponseEvent<ProtocolRequest, ProtocolResponse>),
}

impl From<RelayClientEvent> for BehaviorEvent {
    fn from(event: RelayClientEvent) -> Self {
        BehaviorEvent::RelayClient(event)
    }
}
impl From<ping::Event> for BehaviorEvent {
    fn from(event: ping::Event) -> Self {
        BehaviorEvent::Ping(event)
    }
}
impl From<IdentifyEvent> for BehaviorEvent {
    fn from(event: IdentifyEvent) -> Self {
        BehaviorEvent::Identify(event)
    }
}
impl From<KademliaEvent> for BehaviorEvent {
    fn from(event: KademliaEvent) -> Self {
        BehaviorEvent::Kademlia(event)
    }
}
impl From<mdns::Event> for BehaviorEvent {
    fn from(event: mdns::Event) -> Self {
        BehaviorEvent::Mdns(event)
    }
}
impl From<RequestResponseEvent<ProtocolRequest, ProtocolResponse>> for BehaviorEvent {
    fn from(event: RequestResponseEvent<ProtocolRequest, ProtocolResponse>) -> Self {
        BehaviorEvent::RequestResponse(event)
    }
}

pub async fn start_node(port: u16, data_dir_option: Option<String>) -> Result<(), Box<dyn StdError>> {
    let start_time = Instant::now();

    let data_dir = data_dir_option.unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{}/.cipherstream/node_data_{}", home, port)
    });
    let download_dir = Path::new(&data_dir).join("downloads");

    std::fs::create_dir_all(&data_dir)?;
    std::fs::create_dir_all(&download_dir)?;
    info!("📁 Using data directory: {}", data_dir);
    info!("📁 Downloads directory: {}", download_dir.display());

    let (local_peer_id, local_key) = generate_peer_id();
    info!("🆔 Local peer id: {}", local_peer_id);

    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(local_key.clone()) 
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_quic()
        .with_relay_client(noise::Config::new, yamux::Config::default)?
        .with_behaviour(move |key, relay_client| {
            let store = MemoryStore::new(key.public().to_peer_id());
            let kad_config = KademliaConfig::default();
            let kademlia = KadBehaviour::with_config(key.public().to_peer_id(), store, kad_config);
            let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id()).expect("mDNS creation failed");
            let request_response = request_handler::Behaviour::with_codec(
                FileTransferCodec::default(),
                [(FILE_TRANSFER_PROTO_ID.clone(), ProtocolSupport::Full)].into_iter(),
                RequestResponseConfig::default(),
            );
            let ping = ping::Behaviour::new(ping::Config::new());
            let identify = IdentifyBehaviour::new(IdentifyConfig::new(
                "/cipherstream/id/1.0.0".into(),
                key.public(),
            ));
            
            Ok(Behavior { kademlia, mdns, request_response, ping, identify, relay_client })
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    // Use port 0 for ephemeral port assignment if specified
    let listen_addr_v4 = if port == 0 {
        "/ip4/0.0.0.0/tcp/0".to_string()
    } else {
        format!("/ip4/0.0.0.0/tcp/{}", port)
    };
    
    let listen_addr = listen_addr_v4.parse()?;
    swarm.listen_on(listen_addr)?;

    // Wait for the actual listen address to be reported
    let mut actual_port = port;
    if port == 0 {
        // Wait briefly to get the assigned port
        let timeout = Duration::from_secs(5);
        let mut found_port = false;
        let start = Instant::now();
        
        // Use a loop with select! to wait for address event
        while !found_port && start.elapsed() < timeout {
            tokio::select! {
                event = swarm.select_next_some() => {
                    if let SwarmEvent::NewListenAddr { address, .. } = &event {
                        if let Some(p) = extract_port_from_multiaddr(address) {
                            actual_port = p;
                            found_port = true;
                            info!("🔌 Assigned port: {}", actual_port);
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    // Just a timeout to prevent blocking forever
                }
            }
        }
    }

    info!("🚀 Node started on port {} in {:?}", actual_port, start_time.elapsed());

    loop {
        select! {
            event = swarm.select_next_some() => {
                 match &event {
                    SwarmEvent::NewListenAddr { address, .. } => info!("📡 Listening on {}", address),
                    SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => info!("✅ Connected to {} via {}", peer_id, endpoint.get_remote_address()),
                    SwarmEvent::ConnectionClosed { peer_id, cause, .. } => warn!("❌ Disconnected from {}: {:?}", peer_id, cause),
                    SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => error!("❌ Failed to connect to {:?}: {}", peer_id, error),
                    SwarmEvent::IncomingConnection { local_addr, send_back_addr, .. } => debug!("📥 Incoming connection: {} <- {}", local_addr, send_back_addr),
                    SwarmEvent::IncomingConnectionError { local_addr, send_back_addr, error, .. } => error!("❌ Incoming connection error: {} <- {}: {}", local_addr, send_back_addr, error),
                    SwarmEvent::Behaviour(bev) => {
                         match bev {
                            BehaviorEvent::Mdns(mdns::Event::Discovered(list)) => {
                                for (peer_id, multiaddr) in list {
                                    debug!("mDNS discovered: {} at {}", peer_id, multiaddr);
                                    swarm.behaviour_mut().kademlia.add_address(peer_id, multiaddr.clone());
                                    if *peer_id != local_peer_id {
                                        if let Err(e) = swarm.dial(DialOpts::from(multiaddr.clone())) {
                                            warn!("Failed to dial discovered peer {}: {}", peer_id, e);
                                        }
                                    }
                                }
                            }
                             BehaviorEvent::Identify(IdentifyEvent::Received { peer_id, info, connection_id: _ }) => {
                                debug!("Identify received from {}: Agent={}, Protocols={:?}", peer_id, info.agent_version, info.protocols);
                                for addr in &info.listen_addrs {
                                    swarm.behaviour_mut().kademlia.add_address(peer_id, addr.clone());
                                }
                            }
                            BehaviorEvent::Ping(ping::Event { peer, result, .. }) => {
                                match result {
                                    Ok(rtt) => debug!("🏓 Ping success: {} ({:?})", peer, rtt),
                                    Err(e) => warn!("🏓 Ping failure: {}: {:?}", peer, e),
                                }
                            }
                            BehaviorEvent::RequestResponse(req_resp_event) => {
                                info!("RequestResponse Event: {:?}", req_resp_event);
                                let _download_dir_clone = download_dir.clone();
                                tokio::spawn(async move {
                                    warn!("Handling request response event directly");
                                    // TODO: Implement proper event handling
                                });
                            }
                             BehaviorEvent::Kademlia(kad_event) => {
                                 debug!("Kademlia event: {:?}", kad_event);
                             }
                             BehaviorEvent::RelayClient(relay_event) => {
                                 debug!("RelayClient event: {:?}", relay_event);
                             }
                            _ => {}
                        }
                    }
                    _ => { /* Other swarm events */ }
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(30)) => {
                info!("Performing periodic tasks (e.g., Kademlia bootstrap)");
                match swarm.behaviour_mut().kademlia.bootstrap() {
                    Ok(_) => debug!("Kademlia bootstrap initiated."),
                    Err(e) => warn!("Kademlia bootstrap failed: {:?}", e),
                }
            }
        }
    }
}

// Helper function to extract port from a multiaddr
fn extract_port_from_multiaddr(addr: &Multiaddr) -> Option<u16> {
    use libp2p::multiaddr::Protocol;
    
    for proto in addr.iter() {
        if let Protocol::Tcp(port) = proto {
            return Some(port);
        }
    }
    None
}

// Add this function to handle request response events with our network::Behavior type
async fn handle_req_resp_event(
    event: request_handler::Event<ProtocolRequest, ProtocolResponse>,
    swarm: &mut Swarm<Behavior>,
    download_dir: PathBuf
) -> Result<Option<ProtocolResponse>, AnyhowError> {
    match event {
        request_handler::Event::Message { peer: _, message, connection_id: _ } => {
            match message {
                request_handler::Message::Request { request, channel, .. } => {
                    match request {
                        ProtocolRequest::HandshakeRequest { filename, filesize, transfer_id } => {
                            info!("🤝 Received HandshakeRequest for '{}' ({} bytes, id: {})", 
                                   filename, filesize, transfer_id);
                            
                            // Create download directory if it doesn't exist
                            std::fs::create_dir_all(&download_dir)?;
                            
                            // Always accept for now
                            let response = ProtocolResponse::HandshakeResponse { 
                                accepted: true, 
                                reason: None,
                                transfer_id: Some(transfer_id),
                            };
                            
                            if let Err(e) = swarm.behaviour_mut().request_response.send_response(channel, response) {
                                return Err(anyhow!("Failed to send response: {:?}", e));
                            }
                            Ok(None)
                        },
                        ProtocolRequest::FileChunk { transfer_id, chunk_index, total_chunks, data: _, is_last: _ } => {
                            info!("📦 Received file chunk {} of {} for transfer {}", 
                                 chunk_index, total_chunks, transfer_id);
                            
                            // Handle file chunk, write to disk, etc.
                            
                            // Send response to acknowledge the chunk
                            let response = ProtocolResponse::ChunkResponse { 
                                transfer_id: transfer_id.clone(), 
                                chunk_index,
                                success: true,
                                error: None,
                            };
                            
                            if let Err(e) = swarm.behaviour_mut().request_response.send_response(channel, response) {
                                return Err(anyhow!("Failed to send response: {:?}", e));
                            }
                            Ok(None)
                        },
                        ProtocolRequest::CancelTransfer { transfer_id } => {
                            info!("🛑 Received cancel transfer request for {}", transfer_id);
                            
                            let response = ProtocolResponse::TransferComplete {
                                transfer_id: transfer_id.clone(),
                                success: false,
                                error: Some("Transfer cancelled by sender".to_string()),
                            };
                            
                            if let Err(e) = swarm.behaviour_mut().request_response.send_response(channel, response) {
                                return Err(anyhow!("Failed to send response: {:?}", e));
                            }
                            Ok(None)
                        }
                    }
                },
                request_handler::Message::Response { response, .. } => {
                    // Return the response so the caller can handle it
                    Ok(Some(response))
                }
            }
        },
        request_handler::Event::OutboundFailure { peer, error, .. } => {
            error!("❌ Outbound request failed to {}: {:?}", peer, error);
            Ok(None)
        },
        request_handler::Event::InboundFailure { peer, error, .. } => {
            error!("❌ Inbound request failed from {}: {:?}", peer, error);
            Ok(None)
        },
        request_handler::Event::ResponseSent { peer, .. } => {
            debug!("✅ Response sent to {}", peer);
            Ok(None)
        }
    }
}

// Add these functions to handle file transfers with network::Behavior
async fn send_handshake_request(
    swarm: &mut Swarm<Behavior>,
    peer_id: &PeerId,
    filename: &str,
    file_path: &str,
) -> Result<OutboundRequestId, AnyhowError> {
    info!("🤝 Initiating handshake for '{}' with {}", filename, peer_id);
    
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

    // Send the request
    let request_id = swarm.behaviour_mut().request_response.send_request(peer_id, request);
    info!("📤 Sent HandshakeRequest, request ID: {:?}", request_id);
    
    Ok(request_id)
}

async fn send_file(
    swarm: &mut Swarm<Behavior>,
    peer_id: &PeerId,
    file_path: &str,
    transfer_id: &str,
) -> Result<(), AnyhowError> {
    // Open the file
    let mut file = tokio::fs::File::open(file_path).await?;
    
    // Get file metadata
    let metadata = file.metadata().await?;
    let file_size = metadata.len();
    
    // Calculate number of chunks
    let chunk_size = 1024 * 1024; // 1MB
    let total_chunks = (file_size + chunk_size as u64 - 1) / chunk_size as u64;
    
    info!("📤 Starting file transfer: {}, size: {} bytes, chunks: {}", file_path, file_size, total_chunks);
    
    // Send file in chunks
    let mut offset = 0;
    let mut chunk_index = 0;
    let mut buffer = vec![0; chunk_size];
    
    while offset < file_size {
        // Fix: capture the return value from read()
        tokio::io::AsyncSeekExt::seek(&mut file, std::io::SeekFrom::Start(offset)).await?;
        let n = tokio::io::AsyncReadExt::read(&mut file, &mut buffer[..]).await?;
        
        if n == 0 {
            break; // End of file
        }
        
        // Is this the last chunk?
        let is_last = offset + n as u64 >= file_size;
        
        // Create the file chunk request
        let request = ProtocolRequest::FileChunk {
            transfer_id: transfer_id.to_string(),
            chunk_index,
            total_chunks,
            data: buffer[..n].to_vec(),
            is_last,
        };
        
        // Send the chunk
        info!("📤 Sending chunk {} of {} for transfer {}", chunk_index, total_chunks, transfer_id);
        let _request_id = swarm.behaviour_mut().request_response.send_request(peer_id, request);
        
        // Update offset and chunk index
        offset += n as u64;
        chunk_index += 1;

        // Yield to allow other tasks to run
        tokio::task::yield_now().await;
    }
    
    info!("✅ File transfer complete! Sent {} bytes in {} chunks", file_size, chunk_index);
    
    Ok(())
}

async fn cancel_transfer(
    swarm: &mut Swarm<Behavior>,
    peer_id: &PeerId,
    transfer_id: &str,
) -> Result<OutboundRequestId, AnyhowError> {
    info!("🛑 Cancelling transfer {} with {}", transfer_id, peer_id);
    
    let request = ProtocolRequest::CancelTransfer {
        transfer_id: transfer_id.to_string(),
    };
    
    let request_id = swarm.behaviour_mut().request_response.send_request(peer_id, request);
    Ok(request_id)
}

// Update the start_temp_node_and_send_file function to use these new functions
// Note: libp2p Noise protocol provides transport-level encryption automatically
pub async fn start_temp_node_and_send_file(
    target_peer_id: PeerId,
    file_path_str: String,
    _encrypt: bool, // Deprecated: libp2p Noise provides encryption
    data_dir: Option<String>,
) -> Result<(), AnyhowError> {
    let _start_time = Instant::now();
    let mut handshake_sent = false;
    let mut handshake_request_id = None;
    let transfer_id: Option<String> = None;
    
    let temp_data_dir = data_dir.unwrap_or_else(|| {
        format!(".cipherstream_temp_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs())
    });
    
    std::fs::create_dir_all(&temp_data_dir)?;

    let (local_peer_id, local_key) = generate_peer_id();
    info!("🆔 Temp node peer id: {}", local_peer_id);

    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(local_key.clone()) 
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_quic()
        .with_relay_client(noise::Config::new, yamux::Config::default)?
        .with_behaviour(move |key, relay_client| {
            let store = MemoryStore::new(key.public().to_peer_id());
            let kad_config = KademliaConfig::default();
            let kademlia = KadBehaviour::with_config(key.public().to_peer_id(), store, kad_config);
            let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id()).expect("mDNS creation failed");
            let request_response = request_handler::Behaviour::with_codec(
                FileTransferCodec::default(),
                [(FILE_TRANSFER_PROTO_ID.clone(), ProtocolSupport::Full)].into_iter(),
                RequestResponseConfig::default(),
            );
            let ping = ping::Behaviour::new(ping::Config::new());
            let identify = IdentifyBehaviour::new(IdentifyConfig::new(
                "/cipherstream/id/1.0.0".into(),
                key.public(),
            ));
            
            Ok(Behavior { kademlia, mdns, request_response, ping, identify, relay_client })
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    // Listen on random port
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    // Get the path parts for the filename
    let path = std::path::Path::new(&file_path_str);
    let filename = path.file_name()
        .ok_or_else(|| anyhow!("Invalid filename"))?
        .to_str()
        .ok_or_else(|| anyhow!("Filename contains invalid UTF-8"))?;

    // Prepare a manual connection to the target peer
    let target_peer_addresses = get_peer_addresses(&target_peer_id);
    let mut connected = false;

    let timeout_duration = Duration::from_secs(60); // 60 second timeout
    let timeout_future = tokio::time::sleep(timeout_duration);
    tokio::pin!(timeout_future);

    loop {
        select! {
            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        info!("📡 Listening on {}", address);
                    }
                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        if peer_id == target_peer_id {
                            info!("✅ Connected to target peer: {}", peer_id);
                            connected = true;

                            if !handshake_sent {
                                info!("🤝 Sending handshake request for file: {}", filename);
                                match send_handshake_request(&mut swarm, &target_peer_id, &filename, &file_path_str).await {
                                    Ok(request_id) => {
                                        handshake_sent = true;
                                        handshake_request_id = Some(request_id);
                                    }
                                    Err(e) => {
                                        error!("❌ Failed to send handshake request: {}", e);
                                        return Err(e);
                                    }
                                }
                            } else {
                                debug!("Handshake already sent, request ID: {:?}", handshake_request_id);
                            }
                        }
                    }
                    SwarmEvent::Behaviour(BehaviorEvent::RequestResponse(req_resp_event)) => {
                        let temp_download_dir = PathBuf::from(&temp_data_dir);
                        match handle_req_resp_event(req_resp_event, &mut swarm, temp_download_dir).await {
                            Ok(Some(ProtocolResponse::HandshakeResponse { accepted, reason, transfer_id: resp_transfer_id })) => {
                                if accepted {
                                    if let Some(tid) = resp_transfer_id {
                                        info!("👍 Handshake accepted by peer. Transfer ID: {}", tid);
                                        
                                        info!("🚀 Starting file send process...");
                                        match send_file(&mut swarm, &target_peer_id, &file_path_str, &tid).await {
                                            Ok(()) => {
                                                info!("✅ File transfer process initiated successfully for transfer ID: {}", tid);
                                                let _ = std::fs::remove_dir_all(&temp_data_dir);
                                                return Ok(());
                                            },
                                            Err(e) => {
                                                error!("❌ File transfer failed: {}", e);
                                                let _ = cancel_transfer(&mut swarm, &target_peer_id, &tid).await;
                                                let _ = std::fs::remove_dir_all(&temp_data_dir);
                                                return Err(e);
                                            }
                                        }
                                    } else {
                                        error!("❌ Handshake accepted but no transfer ID received.");
                                        let _ = std::fs::remove_dir_all(&temp_data_dir);
                                        return Err(anyhow!("Handshake accepted without transfer ID"));
                                    }
                                } else {
                                    error!("👎 Handshake rejected by peer: {}", reason.unwrap_or_else(|| "No reason given".to_string()));
                                    let _ = std::fs::remove_dir_all(&temp_data_dir);
                                    return Err(anyhow!("Handshake rejected"));
                                }
                            }
                            Ok(Some(ProtocolResponse::TransferComplete { transfer_id: completed_tid, success, error: err_msg })) => {
                                info!("🏁 Received TransferComplete for {}: Success={}, Error={:?}", completed_tid, success, err_msg);
                                let _ = std::fs::remove_dir_all(&temp_data_dir);
                                if success {
                                    return Ok(());
                                } else {
                                    return Err(anyhow!("Transfer failed on receiver side: {:?}", err_msg));
                                }
                            }
                            Ok(_) => {
                                debug!("Received other RequestResponse event (e.g., ChunkResponse)");
                            }
                            Err(e) => {
                                error!("❌ Error handling request/response event: {}", e);
                            }
                        }
                    }
                    // ... other event handlers remain the same ...
                    
                    // Add a wildcard pattern to handle all other event types
                    _ => {
                        // Ignored - handle other event types as needed
                    }
                }
            }
            () = &mut timeout_future => {
                error!("❌ File transfer timed out after {:?}", timeout_duration);
                if let Some(tid) = transfer_id {
                    let _ = cancel_transfer(&mut swarm, &target_peer_id, &tid).await;
                }
                let _ = std::fs::remove_dir_all(&temp_data_dir);
                return Err(anyhow!("File transfer timed out"));
            }
        }

        // Try to connect to the target peer if we have addresses and aren't connected yet
        if !connected {
            if let Some(addrs) = target_peer_addresses.clone() {
                for addr in addrs {
                    info!("Attempting to dial {} at {}", target_peer_id, addr);
                    if let Err(e) = swarm.dial(DialOpts::from(addr.clone())) {
                        warn!("Failed to dial {}: {}", addr, e);
                    }
                }
            } else {
                // No known addresses for peer, try to use Kademlia DHT to find it
                info!("No known addresses for {}. Trying to find peer via DHT...", target_peer_id);
                swarm.behaviour_mut().kademlia.get_closest_peers(target_peer_id);
            }
        }
    }
}

// Function to get peer addresses (simplified placeholder)
fn get_peer_addresses(_peer_id: &PeerId) -> Option<Vec<Multiaddr>> {
    // In a real implementation, this would look up addresses from discovered peers
    // For now, we'll return None to let the DHT handle discovery
    None
}

