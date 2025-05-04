use cipherstream::protocol::FileMetadata;
use serde_json;

#[test]
fn test_file_metadata_serialization() {
    // Create a file metadata object
    let metadata = FileMetadata {
        filename: "test_file.txt".to_string(),
        size: 1024,
        checksum: "abcdef123456".to_string(),
        encrypted: true,
    };
    
    // Serialize to JSON
    let json = serde_json::to_string(&metadata).unwrap();
    
    // Deserialize back to a struct
    let deserialized: FileMetadata = serde_json::from_str(&json).unwrap();
    
    // Verify fields match
    assert_eq!(deserialized.filename, "test_file.txt");
    assert_eq!(deserialized.size, 1024);
    assert_eq!(deserialized.checksum, "abcdef123456");
    assert_eq!(deserialized.encrypted, true);
}

#[test]
fn test_file_metadata_with_special_chars() {
    // Create a file metadata object with special characters in the filename
    let metadata = FileMetadata {
        filename: "test file with spaces & special chars!.txt".to_string(),
        size: 2048,
        checksum: "123abc".to_string(),
        encrypted: false,
    };
    
    // Serialize to JSON
    let json = serde_json::to_string(&metadata).unwrap();
    
    // Deserialize back to a struct
    let deserialized: FileMetadata = serde_json::from_str(&json).unwrap();
    
    // Verify fields match including the special characters
    assert_eq!(deserialized.filename, "test file with spaces & special chars!.txt");
    assert_eq!(deserialized.size, 2048);
    assert_eq!(deserialized.checksum, "123abc");
    assert_eq!(deserialized.encrypted, false);
}

#[test]
fn test_file_metadata_empty_fields() {
    // Create a file metadata object with minimal data
    let metadata = FileMetadata {
        filename: "".to_string(),
        size: 0,
        checksum: "".to_string(),
        encrypted: false,
    };
    
    // Serialize to JSON
    let json = serde_json::to_string(&metadata).unwrap();
    
    // Deserialize back to a struct
    let deserialized: FileMetadata = serde_json::from_str(&json).unwrap();
    
    // Verify empty fields are preserved
    assert_eq!(deserialized.filename, "");
    assert_eq!(deserialized.size, 0);
    assert_eq!(deserialized.checksum, "");
    assert_eq!(deserialized.encrypted, false);
}

#[test]
fn test_file_metadata_large_size() {
    // Create a file metadata object with a large file size
    let metadata = FileMetadata {
        filename: "large_file.bin".to_string(),
        size: u64::MAX, // Maximum possible file size
        checksum: "large_file_checksum".to_string(),
        encrypted: true,
    };
    
    // Serialize to JSON
    let json = serde_json::to_string(&metadata).unwrap();
    
    // Deserialize back to a struct
    let deserialized: FileMetadata = serde_json::from_str(&json).unwrap();
    
    // Verify the large size is preserved
    assert_eq!(deserialized.filename, "large_file.bin");
    assert_eq!(deserialized.size, u64::MAX);
    assert_eq!(deserialized.checksum, "large_file_checksum");
    assert_eq!(deserialized.encrypted, true);
}

#[test]
fn test_file_metadata_json_structure() {
    // Create a file metadata object
    let metadata = FileMetadata {
        filename: "test.txt".to_string(),
        size: 100,
        checksum: "aabbcc".to_string(),
        encrypted: true,
    };
    
    // Serialize to pretty-printed JSON
    let json = serde_json::to_string_pretty(&metadata).unwrap();
    
    // Check that the JSON contains all expected fields
    assert!(json.contains("\"filename\""));
    assert!(json.contains("\"size\""));
    assert!(json.contains("\"checksum\""));
    assert!(json.contains("\"encrypted\""));
    
    // Check that the values are correctly encoded
    assert!(json.contains("\"test.txt\""));
    assert!(json.contains("100"));
    assert!(json.contains("\"aabbcc\""));
    assert!(json.contains("true"));
} 