use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, Result};
use ring::digest::{Context, SHA256};
use rand::{distributions::Alphanumeric, Rng};

/// Calculate SHA-256 hash of a file
pub async fn sha256_file<P: AsRef<Path>>(path: P) -> Result<String> {
    let mut file = File::open(path).await?;
    let mut context = Context::new(&SHA256);
    let mut buffer = [0u8; 1024 * 64];
    
    loop {
        let count = file.read(&mut buffer).await?;
        if count == 0 {
            break;
        }
        context.update(&buffer[..count]);
    }
    
    let digest = context.finish();
    Ok(hex::encode(digest.as_ref()))
}

/// Format file size in human-readable format
pub fn format_size(size: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut size = size as f64;
    let mut unit_index = 0;
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    format!("{:.2} {}", size, UNITS[unit_index])
}

/// Generate a random alphanumeric string ID
pub fn random_id(length: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
} 