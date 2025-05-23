use clap::{Parser, Subcommand};
use log::{info, error};
use std::path::PathBuf;
use std::error::Error;

// Added for tracing file logging
use tracing_subscriber::{fmt, EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
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
}

// Function to initialize tracing and file logging
// Returns a WorkerGuard that must be kept alive for logs to be written
fn init_logging(log_file_prefix: &str) -> Result<WorkerGuard, Box<dyn Error>> {
    // Create a directory for logs if it doesn't exist
    std::fs::create_dir_all("logs")?;

    let file_appender = tracing_appender::rolling::daily("logs", log_file_prefix);
    let (non_blocking_appender, guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = fmt::layer()
        .with_writer(non_blocking_appender)
        .with_ansi(false); // Don't use ANSI codes in files

    let console_layer = fmt::layer()
        .with_writer(std::io::stdout);

    // Use RUST_LOG env var, default to info for self and warn for others
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,libp2p_swarm=warn"));

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
    
    // Determine log file prefix based on command (basic example)
    // This guard needs to stay in scope, otherwise logs stop writing.
    let _guard = init_logging("cipherstream_node")?;

    let cli = Cli::parse();
    
    match cli.command {
        Commands::Start { port, data_dir } => {
            info!("Starting node on port {}...", port);
            
            // Create application configuration
            let mut config = AppConfig::default();
            config.default_port = port;
            config.data_directory = data_dir.clone();
            config.download_directory = format!("{}/downloads", data_dir);
            
            // Initialize application service
            let app_service = ApplicationService::new(config.clone()).await?;
            info!("📁 Using data directory: {}", app_service.config().data_directory);
            
            // Initialize event publisher
            let event_publisher = std::sync::Arc::new(InMemoryEventPublisher::new());
            
            // Initialize libp2p network service
            let network_service = LibP2pNetworkService::new(
                std::sync::Arc::new(config),
                event_publisher
            ).await.map_err(|e| format!("Failed to create network service: {}", e))?;
            
            let peer_id = network_service.local_peer_id();
            info!("🆔 Local peer id: {}", peer_id);
            
            // Start the network service
            network_service.start_listening(port).await
                .map_err(|e| format!("Failed to start listening: {}", e))?;
            info!("🚀 Node started on port {}", port);
            
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
    }
    
    Ok(())
}
