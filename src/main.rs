use clap::{Parser, Subcommand};
use log::{info, error};
use std::path::PathBuf;
use std::error::Error;
use libp2p::PeerId;
use std::str::FromStr;

// Added for tracing file logging
use tracing_subscriber::{fmt, EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_appender::non_blocking::WorkerGuard;

pub mod crypto;
pub mod discovery;
pub mod network;
pub mod protocol;
pub mod utils;
pub mod file_transfer;

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
        
        /// Encrypt the file before sending
        #[arg(short, long)]
        encrypt: bool,
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
            if let Err(e) = network::start_node(port, Some(data_dir)).await {
                error!("Node failed to start: {}", e);
            }
        }
        Commands::Send { file, peer, encrypt } => {
            if !file.exists() {
                error!("File does not exist: {:?}", file);
                return Err("File not found".into());
            }
            
            // Parse peer ID
            let peer_id = match PeerId::from_str(&peer) {
                Ok(id) => id,
                Err(_) => {
                    error!("Invalid peer ID format: {}", peer);
                    return Err("Invalid peer ID format".into());
                }
            };
            
            // We need to start a temporary node to send the file
            println!("Starting temporary node to send file...");
            
            // First generate a peer ID for our temporary node
            let (local_peer_id, _local_key) = network::generate_peer_id();
            println!("Temporary node ID: {}", local_peer_id);
            
            // Start a swarm with a single purpose - to send this file
            let file_path = file.to_string_lossy().to_string();
            
            println!("Initiating file transfer for {} to {}", file_path, peer_id);
            println!("Encryption: {}", if encrypt { "enabled" } else { "disabled" });
            
            // Create a temp node and send the file
            // Use a temporary data directory for the sending node
            let temp_data_dir = format!(".cipherstream_temp_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
            network::start_temp_node_and_send_file(peer_id, file_path, encrypt, Some(temp_data_dir)).await?;
            
            println!("File transfer command completed. Check logs for status.");
        }
        Commands::Connect { peer } => {
            // Parse peer ID
            let peer_id = match PeerId::from_str(&peer) {
                Ok(id) => id,
                Err(_) => {
                    error!("Invalid peer ID format: {}", peer);
                    return Err("Invalid peer ID format".into());
                }
            };
            
            info!("Connecting to peer: {}", peer_id);
            discovery::dial_peer(peer_id)?;
            println!("Connection initiated. Check logs for status.");
        }
        Commands::Peers => {
            info!("Listing peers...");
            let peers = discovery::get_discovered_peers();
            
            if peers.is_empty() {
                println!("No peers discovered yet. Start by running the Start command.");
            } else {
                println!("Discovered peers:");
                for (peer_id, addrs) in peers {
                    println!("Peer: {} at:", peer_id);
                    for addr in addrs {
                        println!("  - {}", addr);
                    }
                }
            }
        }
    }
    
    Ok(())
}
