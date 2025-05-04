use cipherstream::file_transfer::request_handler::{FileTransferCodec, FileTransferProtocol};
use cipherstream::file_transfer::types::{ProtocolRequest, ProtocolResponse};
use libp2p::request_response::Codec;
use futures::io::Cursor;
use std::io;
use async_std::task;

#[test]
fn test_codec_protocol_creation() {
    let protocol = FileTransferProtocol::new();
    assert_eq!(protocol.as_ref(), "/cipherstream/file-transfer/1.0.0");
    
    let codec = FileTransferCodec::default();
    // Just verify we can create the codec
    assert!(true);
}

#[test]
fn test_codec_read_write_request() {
    let protocol = FileTransferProtocol::new();
    let mut codec = FileTransferCodec::default();
    
    // Create a test request
    let request = ProtocolRequest::HandshakeRequest {
        filename: "test.txt".to_string(),
        filesize: 1024,
        encrypted: true,
        transfer_id: "test-id-1".to_string(),
    };
    
    // Use a buffer to simulate the IO
    let mut buffer = Vec::new();
    let mut cursor = Cursor::new(&mut buffer);
    
    // Write the request to the buffer
    let write_result = task::block_on(async {
        codec.write_request(&protocol, &mut cursor, request.clone()).await
    });
    
    assert!(write_result.is_ok());
    
    // Reset the cursor to read from the beginning
    let mut read_cursor = Cursor::new(&buffer);
    
    // Read the request from the buffer
    let read_result = task::block_on(async {
        codec.read_request(&protocol, &mut read_cursor).await
    });
    
    assert!(read_result.is_ok());
    let decoded_request = read_result.unwrap();
    
    // Verify the decoded request matches the original
    match (decoded_request, request) {
        (
            ProtocolRequest::HandshakeRequest { filename: f1, filesize: s1, encrypted: e1, transfer_id: t1 },
            ProtocolRequest::HandshakeRequest { filename: f2, filesize: s2, encrypted: e2, transfer_id: t2 }
        ) => {
            assert_eq!(f1, f2);
            assert_eq!(s1, s2);
            assert_eq!(e1, e2);
            assert_eq!(t1, t2);
        },
        _ => panic!("Decoded to wrong variant"),
    }
}

#[test]
fn test_codec_read_write_response() {
    let protocol = FileTransferProtocol::new();
    let mut codec = FileTransferCodec::default();
    
    // Create a test response
    let response = ProtocolResponse::HandshakeResponse {
        accepted: true,
        reason: None,
        transfer_id: Some("test-id-1".to_string()),
    };
    
    // Use a buffer to simulate the IO
    let mut buffer = Vec::new();
    let mut cursor = Cursor::new(&mut buffer);
    
    // Write the response to the buffer
    let write_result = task::block_on(async {
        codec.write_response(&protocol, &mut cursor, response.clone()).await
    });
    
    assert!(write_result.is_ok());
    
    // Reset the cursor to read from the beginning
    let mut read_cursor = Cursor::new(&buffer);
    
    // Read the response from the buffer
    let read_result = task::block_on(async {
        codec.read_response(&protocol, &mut read_cursor).await
    });
    
    assert!(read_result.is_ok());
    let decoded_response = read_result.unwrap();
    
    // Verify the decoded response matches the original
    match (decoded_response, response) {
        (
            ProtocolResponse::HandshakeResponse { accepted: a1, reason: r1, transfer_id: t1 },
            ProtocolResponse::HandshakeResponse { accepted: a2, reason: r2, transfer_id: t2 }
        ) => {
            assert_eq!(a1, a2);
            assert_eq!(r1, r2);
            assert_eq!(t1, t2);
        },
        _ => panic!("Decoded to wrong variant"),
    }
}

#[test]
fn test_codec_file_chunk_roundtrip() {
    let protocol = FileTransferProtocol::new();
    let mut codec = FileTransferCodec::default();
    
    // Create a file chunk request with binary data
    let chunk_data = vec![0, 1, 2, 3, 4, 5, 255, 254, 253];
    let request = ProtocolRequest::FileChunk {
        transfer_id: "chunk-test-id".to_string(),
        chunk_index: 42,
        total_chunks: 100,
        data: chunk_data.clone(),
        is_last: false,
    };
    
    // Use a buffer to simulate the IO
    let mut buffer = Vec::new();
    let mut cursor = Cursor::new(&mut buffer);
    
    // Write the request to the buffer
    let write_result = task::block_on(async {
        codec.write_request(&protocol, &mut cursor, request.clone()).await
    });
    
    assert!(write_result.is_ok());
    
    // Reset the cursor to read from the beginning
    let mut read_cursor = Cursor::new(&buffer);
    
    // Read the request from the buffer
    let read_result = task::block_on(async {
        codec.read_request(&protocol, &mut read_cursor).await
    });
    
    assert!(read_result.is_ok());
    let decoded_request = read_result.unwrap();
    
    // Verify the decoded request matches the original, especially the binary data
    match decoded_request {
        ProtocolRequest::FileChunk { transfer_id, chunk_index, total_chunks, data, is_last } => {
            assert_eq!(transfer_id, "chunk-test-id");
            assert_eq!(chunk_index, 42);
            assert_eq!(total_chunks, 100);
            assert_eq!(data, chunk_data);
            assert_eq!(is_last, false);
        },
        _ => panic!("Decoded to wrong variant"),
    }
}

#[test]
fn test_codec_large_data_handling() {
    let protocol = FileTransferProtocol::new();
    let mut codec = FileTransferCodec::default();
    
    // Create a large chunk of data (1MB)
    let large_data = vec![0x55; 1024 * 1024];
    
    // Create a file chunk request with the large data
    let request = ProtocolRequest::FileChunk {
        transfer_id: "large-data-test".to_string(),
        chunk_index: 1,
        total_chunks: 10,
        data: large_data.clone(),
        is_last: false,
    };
    
    // Use a buffer to simulate the IO
    let mut buffer = Vec::new();
    let mut cursor = Cursor::new(&mut buffer);
    
    // Write the request to the buffer
    let write_result = task::block_on(async {
        codec.write_request(&protocol, &mut cursor, request).await
    });
    
    assert!(write_result.is_ok());
    
    // Reset the cursor to read from the beginning
    let mut read_cursor = Cursor::new(&buffer);
    
    // Read the request from the buffer
    let read_result = task::block_on(async {
        codec.read_request(&protocol, &mut read_cursor).await
    });
    
    assert!(read_result.is_ok());
    let decoded_request = read_result.unwrap();
    
    // Verify the large data was properly decoded
    match decoded_request {
        ProtocolRequest::FileChunk { data, .. } => {
            assert_eq!(data.len(), 1024 * 1024);
            assert_eq!(data, large_data);
        },
        _ => panic!("Decoded to wrong variant"),
    }
} 