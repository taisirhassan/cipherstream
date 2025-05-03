use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, Result as IoResult};
use ring::digest::{Context, SHA256};
use rand::{distributions::Alphanumeric, Rng};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use std::error::Error;

/// Calculate SHA-256 hash of a file
pub async fn sha256_file<P: AsRef<Path>>(path: P) -> IoResult<String> {
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

/// Generate a random unique ID for transfers and other operations
pub fn generate_id() -> String {
    Uuid::new_v4().to_string()
}

/// Get the current Unix timestamp in seconds
pub fn get_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs()
}

/// Format a file size in human-readable form
pub fn format_size(size: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < units.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", size as u64, units[unit_index])
    } else {
        format!("{:.2} {}", size, units[unit_index])
    }
}

/// Get the filename from a path
pub fn get_filename(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|os_str| os_str.to_str())
        .map(String::from)
}

/// Calculate the number of chunks for a file given a chunk size
pub fn calculate_chunks(file_size: u64, chunk_size: usize) -> u64 {
    (file_size + chunk_size as u64 - 1) / chunk_size as u64
}

/// Generate a random alphanumeric string ID
pub fn random_id(length: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

/// Check if a file exists and get its size
pub async fn check_file(path: &Path) -> Result<u64, Box<dyn Error>> {
    let metadata = tokio::fs::metadata(path).await?;
    if !metadata.is_file() {
        return Err("Not a file".into());
    }
    Ok(metadata.len())
}

/// Create directory if it doesn't exist
pub async fn ensure_dir(path: &Path) -> Result<(), Box<dyn Error>> {
    if !path.exists() {
        tokio::fs::create_dir_all(path).await?;
    }
    Ok(())
}

/// Simple test to validate Utils functionality
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_generate_id() {
        let id1 = generate_id();
        let id2 = generate_id();
        assert_ne!(id1, id2);
        assert_eq!(id1.len(), 36); // UUID length
    }
} 