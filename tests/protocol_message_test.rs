use cipherstream::file_transfer::types::{ProtocolRequest, ProtocolResponse};
use bincode::{config, Encode, Decode};

#[test]
fn test_protocol_request_serialization() {
    // Test HandshakeRequest
    let handshake = ProtocolRequest::HandshakeRequest {
        filename: "test.txt".to_string(),
        filesize: 1024,
        encrypted: true,
        transfer_id: "test-id-1".to_string(),
    };
    
    // Serialize using bincode
    let config = config::standard();
    let encoded: Vec<u8> = bincode::encode_to_vec(&handshake, config).unwrap();
    
    // Deserialize and verify
    let (decoded, _): (ProtocolRequest, usize) = bincode::decode_from_slice(&encoded, config).unwrap();
    
    match decoded {
        ProtocolRequest::HandshakeRequest { filename, filesize, encrypted, transfer_id } => {
            assert_eq!(filename, "test.txt");
            assert_eq!(filesize, 1024);
            assert_eq!(encrypted, true);
            assert_eq!(transfer_id, "test-id-1");
        },
        _ => panic!("Decoded to wrong variant"),
    }
    
    // Test FileChunk
    let chunk = ProtocolRequest::FileChunk {
        transfer_id: "test-id-2".to_string(),
        chunk_index: 5,
        total_chunks: 10,
        data: vec![1, 2, 3, 4, 5],
        is_last: false,
    };
    
    let encoded: Vec<u8> = bincode::encode_to_vec(&chunk, config).unwrap();
    let (decoded, _): (ProtocolRequest, usize) = bincode::decode_from_slice(&encoded, config).unwrap();
    
    match decoded {
        ProtocolRequest::FileChunk { transfer_id, chunk_index, total_chunks, data, is_last } => {
            assert_eq!(transfer_id, "test-id-2");
            assert_eq!(chunk_index, 5);
            assert_eq!(total_chunks, 10);
            assert_eq!(data, vec![1, 2, 3, 4, 5]);
            assert_eq!(is_last, false);
        },
        _ => panic!("Decoded to wrong variant"),
    }
    
    // Test CancelTransfer
    let cancel = ProtocolRequest::CancelTransfer {
        transfer_id: "test-id-3".to_string(),
    };
    
    let encoded: Vec<u8> = bincode::encode_to_vec(&cancel, config).unwrap();
    let (decoded, _): (ProtocolRequest, usize) = bincode::decode_from_slice(&encoded, config).unwrap();
    
    match decoded {
        ProtocolRequest::CancelTransfer { transfer_id } => {
            assert_eq!(transfer_id, "test-id-3");
        },
        _ => panic!("Decoded to wrong variant"),
    }
}

#[test]
fn test_protocol_response_serialization() {
    // Test HandshakeResponse - accepted
    let response_accepted = ProtocolResponse::HandshakeResponse {
        accepted: true,
        reason: None,
        transfer_id: Some("test-id-1".to_string()),
    };
    
    let config = config::standard();
    let encoded: Vec<u8> = bincode::encode_to_vec(&response_accepted, config).unwrap();
    let (decoded, _): (ProtocolResponse, usize) = bincode::decode_from_slice(&encoded, config).unwrap();
    
    match decoded {
        ProtocolResponse::HandshakeResponse { accepted, reason, transfer_id } => {
            assert_eq!(accepted, true);
            assert_eq!(reason, None);
            assert_eq!(transfer_id, Some("test-id-1".to_string()));
        },
        _ => panic!("Decoded to wrong variant"),
    }
    
    // Test HandshakeResponse - rejected
    let response_rejected = ProtocolResponse::HandshakeResponse {
        accepted: false,
        reason: Some("File already exists".to_string()),
        transfer_id: None,
    };
    
    let encoded: Vec<u8> = bincode::encode_to_vec(&response_rejected, config).unwrap();
    let (decoded, _): (ProtocolResponse, usize) = bincode::decode_from_slice(&encoded, config).unwrap();
    
    match decoded {
        ProtocolResponse::HandshakeResponse { accepted, reason, transfer_id } => {
            assert_eq!(accepted, false);
            assert_eq!(reason, Some("File already exists".to_string()));
            assert_eq!(transfer_id, None);
        },
        _ => panic!("Decoded to wrong variant"),
    }
    
    // Test ChunkResponse
    let chunk_response = ProtocolResponse::ChunkResponse {
        transfer_id: "test-id-2".to_string(),
        chunk_index: 3,
        success: true,
        error: None,
    };
    
    let encoded: Vec<u8> = bincode::encode_to_vec(&chunk_response, config).unwrap();
    let (decoded, _): (ProtocolResponse, usize) = bincode::decode_from_slice(&encoded, config).unwrap();
    
    match decoded {
        ProtocolResponse::ChunkResponse { transfer_id, chunk_index, success, error } => {
            assert_eq!(transfer_id, "test-id-2");
            assert_eq!(chunk_index, 3);
            assert_eq!(success, true);
            assert_eq!(error, None);
        },
        _ => panic!("Decoded to wrong variant"),
    }
    
    // Test TransferComplete
    let complete = ProtocolResponse::TransferComplete {
        transfer_id: "test-id-3".to_string(),
        success: true,
        error: None,
    };
    
    let encoded: Vec<u8> = bincode::encode_to_vec(&complete, config).unwrap();
    let (decoded, _): (ProtocolResponse, usize) = bincode::decode_from_slice(&encoded, config).unwrap();
    
    match decoded {
        ProtocolResponse::TransferComplete { transfer_id, success, error } => {
            assert_eq!(transfer_id, "test-id-3");
            assert_eq!(success, true);
            assert_eq!(error, None);
        },
        _ => panic!("Decoded to wrong variant"),
    }
    
    // Test failed transfer
    let failed = ProtocolResponse::TransferComplete {
        transfer_id: "test-id-4".to_string(),
        success: false,
        error: Some("Connection lost".to_string()),
    };
    
    let encoded: Vec<u8> = bincode::encode_to_vec(&failed, config).unwrap();
    let (decoded, _): (ProtocolResponse, usize) = bincode::decode_from_slice(&encoded, config).unwrap();
    
    match decoded {
        ProtocolResponse::TransferComplete { transfer_id, success, error } => {
            assert_eq!(transfer_id, "test-id-4");
            assert_eq!(success, false);
            assert_eq!(error, Some("Connection lost".to_string()));
        },
        _ => panic!("Decoded to wrong variant"),
    }
} 