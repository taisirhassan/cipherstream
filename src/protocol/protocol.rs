use std::path::PathBuf;
use std::io::SeekFrom;
use libp2p::swarm::Swarm;
use serde::{Serialize, Deserialize};
use tokio::{fs::File, io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt}};
use indicatif::{ProgressBar, ProgressStyle};
use futures::prelude::*;

use crate::crypto;
use crate::discovery;

const CHUNK_SIZE: usize = 1024 * 64; // 64KB chunks

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    FileRequest {
        file_name: String,
        file_size: u64,
        chunks: u64,
        hash: String,
    },
    FileAccept {
        transfer_id: String,
    },
    FileReject {
        reason: String,
    },
    FileChunk {
        transfer_id: String,
        chunk_index: u64,
        data: Vec<u8>,
        signature: Vec<u8>,
    },
    FileComplete {
        transfer_id: String,
    },
}

/// Send a file to a peer
pub async fn send_file(file_path: PathBuf, peer_id: String) -> Result<(), Box<dyn std::error::Error>> {
    // Open the file
    let mut file = File::open(&file_path).await?;

    // Get metadata
    let metadata = file.metadata().await?;
    let file_size = metadata.len();
    let chunks = (file_size + CHUNK_SIZE as u64 - 1) / CHUNK_SIZE as u64;

    // Generate transfer ID and keys
    let transfer_id = hex::encode(crypto::generate_key()?);
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).await?;
    let hash = hex::encode(ring::digest::digest(&ring::digest::SHA256, &buffer));
    file.seek(SeekFrom::Start(0)).await?;

    let key = crypto::generate_key()?;
    let (private_key, _public_key) = crypto::generate_signing_keypair()?;

    // Connect to peer
    let mut peer = discovery::connect_to_peer(&peer_id).await?;

    // Progress bar
    let pb = ProgressBar::new(file_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .unwrap());

    // Send request
    let request = Message::FileRequest { file_name: file_path.file_name().unwrap().to_string_lossy().to_string(), file_size, chunks, hash };
    let req_bytes = serde_json::to_vec(&request)?;
    peer.write_all(&req_bytes).await?;

    // Await response
    let mut resp_buf = [0u8; 1024];
    let n = peer.read(&mut resp_buf).await?;
    let response: Message = serde_json::from_slice(&resp_buf[..n])?;

    match response {
        Message::FileAccept { transfer_id: tid } => {
            println!("Transfer accepted with ID: {}", tid);
            let mut buf = vec![0u8; CHUNK_SIZE];
            for idx in 0..chunks {
                let n = file.read(&mut buf).await?;
                if n == 0 { break; }
                let encrypted = crypto::encrypt(&buf[..n], &key)?;
                let signature = crypto::sign_message(&encrypted, &private_key)?;
                let chunk = Message::FileChunk { transfer_id: tid.clone(), chunk_index: idx, data: encrypted, signature };
                let chunk_bytes = serde_json::to_vec(&chunk)?;
                peer.write_all(&chunk_bytes).await?;
                pb.inc(n as u64);
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
            let complete = Message::FileComplete { transfer_id: tid.clone() };
            let complete_bytes = serde_json::to_vec(&complete)?;
            peer.write_all(&complete_bytes).await?;
            pb.finish_with_message("Transfer complete");
        }
        Message::FileReject { reason } => {
            println!("Transfer rejected: {}", reason);
        }
        _ => { println!("Unexpected response"); }
    }
    Ok(())
}

/// Handle incoming behavior events
pub async fn handle_behavior_event(
    swarm: &mut Swarm<discovery::Behavior>,
    event: discovery::BehaviorEvent,
) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Re-implement file transfer event handling using a dedicated protocol/behaviour
    /*
    if let discovery::BehaviorEvent::IncomingFile(_peer, mut stream) = event {
        let mut buf = [0u8; 8192];
        let n = stream.read(&mut buf).await?;
        let message: Message = serde_json::from_slice(&buf[..n])?;
        if let Message::FileRequest { file_name, file_size, chunks, hash: _ } = message {
            println!("Incoming file: {} ({} bytes)", file_name, file_size);
            println!("Accept file? [y/n]");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if input.trim().eq_ignore_ascii_case("y") {
                let tid = hex::encode(crypto::generate_key()?);
                let accept = Message::FileAccept { transfer_id: tid.clone() };
                stream.write_all(&serde_json::to_vec(&accept)?).await?;
                let mut file = File::create(&file_name).await?;
                let pb = ProgressBar::new(file_size);
                pb.set_style(ProgressStyle::default_bar()
                    .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})").unwrap());
                let mut received = 0;
                while received < chunks {
                    let mut cbuf = [0u8; 1024 * 1024];
                    let n = stream.read(&mut cbuf).await?;
                    if n == 0 { break; }
                    let msg: Message = serde_json::from_slice(&cbuf[..n])?;
                    if let Message::FileChunk { transfer_id: id, chunk_index, data, signature: _ } = msg {
                        // TODO: Verify signature and decrypt data
                        if id != tid || chunk_index != received { continue; }
                        file.write_all(&data).await?;
                        pb.inc(data.len() as u64);
                        received += 1;
                    } else if let Message::FileComplete { transfer_id: id } = msg {
                        if id == tid { pb.finish_with_message("Transfer complete"); break; }
                    }
                }
                println!("File saved to {}", file_name);
            } else {
                let reject = Message::FileReject { reason: "User rejected transfer".to_string() };
                stream.write_all(&serde_json::to_vec(&reject)?).await?;
            }
        }
    }
    */
    match event {
        discovery::BehaviorEvent::Mdns(mdns_event) => {
            // Log mdns events if needed
        }
        discovery::BehaviorEvent::Kademlia(kad_event) => {
             // Log kad events if needed
        }
        // Handle other potential BehaviorEvents here
    }
    Ok(())
} 