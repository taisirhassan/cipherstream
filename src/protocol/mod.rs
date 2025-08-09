// Re-export from file_transfer for backward compatibility
pub use crate::file_transfer::types::FileMetadata;

// Protocol constants and utilities
pub const PROTOCOL_VERSION: &str = "1.0.0";
pub const PROTOCOL_ID: &str = "/cipherstream/file-transfer/1.0.0";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_transfer::types::{ProtocolRequest, ProtocolResponse};

    #[test]
    fn test_protocol_id_constants() {
        assert_eq!(PROTOCOL_ID, "/cipherstream/file-transfer/1.0.0");
        assert_eq!(PROTOCOL_VERSION, "1.0.0");
    }

    #[test]
    fn test_protocol_request_serialization() {
        let req = ProtocolRequest::HandshakeRequest {
            filename: "test.txt".to_string(),
            filesize: 1024,
            transfer_id: "abc123".to_string(),
        };

        // Basic sanity check that the request is constructed properly
        match req {
            ProtocolRequest::HandshakeRequest {
                filename,
                filesize,
                transfer_id,
            } => {
                assert_eq!(filename, "test.txt");
                assert_eq!(filesize, 1024);
                assert_eq!(transfer_id, "abc123");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_protocol_response_serialization() {
        let resp = ProtocolResponse::HandshakeResponse {
            accepted: true,
            reason: None,
            transfer_id: Some("abc123".to_string()),
        };

        // Basic sanity check that the response is constructed properly
        match resp {
            ProtocolResponse::HandshakeResponse {
                accepted,
                reason,
                transfer_id,
            } => {
                assert!(accepted);
                assert_eq!(reason, None);
                assert_eq!(transfer_id, Some("abc123".to_string()));
            }
            _ => panic!("Wrong variant"),
        }
    }
}
