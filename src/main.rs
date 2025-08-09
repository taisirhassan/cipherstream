use clap::{Parser, Subcommand};
use tracing::{info, error};
use std::path::PathBuf;
use std::error::Error;
use std::time::Duration;

// Added for tracing file logging
use tracing_subscriber::{fmt, EnvFilter, layer::SubscriberExt, util::SubscriberInitExt, Layer};
use tracing_appender::non_blocking::WorkerGuard;

// Use new modular structure
use cipherstream::{
    infrastructure::{AppConfig, LibP2pNetworkService, CryptoService, InMemoryEventPublisher},
    application::ApplicationService,
    core::{domain::PeerId, traits::NetworkService},
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Reduce console logging noise (overrides env log level to warnings)
    #[arg(long, global = true, default_value_t = false)]
    quiet: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a new peer node
    Start {
        /// Optional port to listen on
        #[arg(short, long, default_value_t = 8000)]
        port: u16,
        
        /// Optional data directory for storing node data
        #[arg(long, default_value = ".cipherstream")]
        data_dir: String,
    },
    /// Send a file to a peer
    Send {
        /// Path to the file to send
        #[arg(short, long)]
        file: PathBuf,
        
        /// Peer ID to send to
        #[arg(short, long)]
        peer: String,
    },
    /// Connect to a specific peer
    Connect {
        /// Peer ID to connect to
        #[arg(short, long)]
        peer: String,
    },
    /// List discovered peers
    Peers,
    /// Discover peers for a short period and print events
    Discover {
        /// Duration in seconds to listen for discovery events
        #[arg(short = 'd', long = "duration", default_value_t = 5)]
        duration_secs: u64,
        /// Port to bind temporarily (helpful if no node is running)
        #[arg(short, long, default_value_t = 8000)]
        port: u16,
    },
}

// Function to initialize tracing and file logging
// Returns a WorkerGuard that must be kept alive for logs to be written
fn init_logging(log_file_prefix: &str, quiet: bool) -> Result<WorkerGuard, Box<dyn Error>> {
    // Create a directory for logs if it doesn't exist
    std::fs::create_dir_all("logs")?;

    // File rotation policy
    let roll_env = std::env::var("CIPHERSTREAM_LOG_ROLL").unwrap_or_else(|_| "daily".to_string());
    let file_appender = match roll_env.as_str() {
        "hourly" => tracing_appender::rolling::hourly("logs", log_file_prefix),
        _ => tracing_appender::rolling::daily("logs", log_file_prefix),
    };
    let (non_blocking_appender, guard) = tracing_appender::non_blocking(file_appender);

    // File format (text or json)
    let file_json = std::env::var("CIPHERSTREAM_LOG_FILE_FORMAT").ok().as_deref() == Some("json");
    let file_layer = if file_json {
        fmt::layer().json().with_writer(non_blocking_appender).with_ansi(false).boxed()
    } else {
        fmt::layer().with_writer(non_blocking_appender).with_ansi(false).boxed()
    };

    let console_json = std::env::var("CIPHERSTREAM_LOG_FORMAT").ok().as_deref() == Some("json");
    // Build console layer with a uniform type by boxing the layer
    let console_layer = if console_json {
        fmt::layer().json().with_writer(std::io::stdout).boxed()
    } else {
        fmt::layer().with_writer(std::io::stdout).boxed()
    };

    // Determine log level with priority:
    // 1) quiet flag -> force warnings
    // 2) CIPHERSTREAM_LOG_LEVEL env var
    // 3) RUST_LOG env var
    // 4) sensible default
    let filter = if quiet {
        EnvFilter::new("warn,libp2p_swarm=warn")
    } else if let Ok(level) = std::env::var("CIPHERSTREAM_LOG_LEVEL") {
        EnvFilter::new(format!("{level},libp2p_swarm=warn"))
    } else {
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info,libp2p_swarm=warn"))
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(console_layer)
        .with(file_layer)
        .init();

    Ok(guard)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // env_logger::init(); // Replaced by tracing setup
    let cli = Cli::parse();

    // Determine log file prefix based on command (basic example)
    // This guard needs to stay in scope, otherwise logs stop writing.
    let _guard = init_logging("cipherstream_node", cli.quiet)?;
    
    match cli.command {
        Commands::Start { port, data_dir } => {
            info!("Starting node on port {}...", port);
            
            // Create application configuration
            let config = AppConfig {
                default_port: port,
                data_directory: data_dir.clone(),
                download_directory: format!("{}/downloads", data_dir),
                ..AppConfig::default()
            };
            
            // Initialize application service
            let app_service = ApplicationService::new(config.clone()).await?;
            info!("Using data directory: {}", app_service.config().data_directory);
            
            // Initialize event publisher
            let event_publisher = std::sync::Arc::new(InMemoryEventPublisher::new());
            
            // Initialize libp2p network service
            let network_service = LibP2pNetworkService::new(
                std::sync::Arc::new(config),
                event_publisher
            ).await.map_err(|e| format!("Failed to create network service: {}", e))?;
            
            let peer_id = network_service.local_peer_id();
            info!("Local peer id: {}", peer_id);
            
            // Start the network service
            network_service.start_listening(port).await
                .map_err(|e| format!("Failed to start listening: {}", e))?;
            info!("Node started on port {}", port);
            
            // Keep the process running
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                info!("Node is running...");
            }
        }
        Commands::Send { file, peer } => {
            if !file.exists() {
                error!("File does not exist: {:?}", file);
                return Err("File not found".into());
            }
            
            // Parse peer ID using new modular structure
            let peer_id = PeerId::from_string(peer);
            
            info!("File transfer functionality will be implemented with new modular architecture");
            info!("Target peer: {}", peer_id.as_str());
            info!("File: {:?}", file);
            
            // Generate hash of file using new crypto service
            let file_hash = CryptoService::compute_file_hash(&file).await
                .map_err(|e| format!("Failed to compute file hash: {}", e))?;
            info!("File hash: {}", file_hash);
            
            println!("File transfer command prepared (implementation pending with new architecture).");
        }
        Commands::Connect { peer } => {
            // Parse peer ID using new structure
            let peer_id = PeerId::from_string(peer);
            
            info!("Connecting to peer: {}", peer_id.as_str());
            println!("Connection functionality will be implemented with new modular architecture.");
        }
        Commands::Peers => {
            info!("Listing peers...");
            
            // In the new architecture, peer discovery would be done through the running network service
            // For now, we'll show a message about how to use the new system
            println!("Peer discovery is available when the node is running.");
            println!("To see connected peers, start a node with: cargo run -- start --port 8000");
            println!("The node will automatically discover and connect to other peers in the network.");
        }
        Commands::Discover { duration_secs, port } => {
            info!("Discovering peers for {} seconds on port {}...", duration_secs, port);

            // Minimal ephemeral setup to leverage the network service
            let config = AppConfig { default_port: port, ..AppConfig::default() };
            let event_publisher = std::sync::Arc::new(InMemoryEventPublisher::new());
            let network_service = LibP2pNetworkService::new(
                std::sync::Arc::new(config),
                event_publisher,
            )
            .await
            .map_err(|e| format!("Failed to create network service: {}", e))?;

            network_service
                .start_listening(port)
                .await
                .map_err(|e| format!("Failed to start listening: {}", e))?;

            let events = network_service
                .collect_events_for(Duration::from_secs(duration_secs))
                .await;

            println!("Discovered {} events:", events.len());
            for ev in events {
                match ev {
                    cipherstream::infrastructure::network::NetworkEvent::PeerConnected(pid) => {
                        println!("Peer connected: {}", pid);
                    }
                    cipherstream::infrastructure::network::NetworkEvent::PeerDisconnected(pid) => {
                        println!("Peer disconnected: {}", pid);
                    }
                    cipherstream::infrastructure::network::NetworkEvent::GossipMessage { from, topic, .. } => {
                        println!("Gossip message from {} on topic {}", from, topic);
                    }
                    cipherstream::infrastructure::network::NetworkEvent::FileTransferRequest { from, .. } => {
                        println!("File transfer request from {}", from);
                    }
                    cipherstream::infrastructure::network::NetworkEvent::FileTransferResponse { from, .. } => {
                        println!("File transfer response from {}", from);
                    }
                }
            }
        }
    }
    
    Ok(())
}
