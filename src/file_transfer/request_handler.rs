use libp2p::request_response::{Codec, ProtocolSupport};
use async_trait::async_trait;
use futures::io::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt};
use std::io;
use super::types::{ProtocolRequest, ProtocolResponse};
use bincode::config;

/// Protocol identifier for file transfer
#[derive(Debug, Clone)]
pub struct FileTransferProtocol;

impl FileTransferProtocol {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileTransferProtocol {
    fn default() -> Self { Self::new() }
}

impl AsRef<str> for FileTransferProtocol {
    fn as_ref(&self) -> &str {
        "/cipherstream/file-transfer/1.0.0"
    }
}

impl From<FileTransferProtocol> for ProtocolSupport {
    fn from(_: FileTransferProtocol) -> Self {
        ProtocolSupport::Full
    }
}

/// Codec for encoding/decoding file transfer messages
#[derive(Default, Debug, Clone)]
pub struct FileTransferCodec;

// To protect against unbounded memory allocation, limit the maximum frame size.
// The project targets ~1MB chunks, so a 2MB cap provides headroom.
const MAX_FRAME_SIZE: usize = 2 * 1024 * 1024; // 2 MiB
// Per-message-type budgets (handshake is small, chunk is larger)
const MAX_HANDSHAKE_SIZE: usize = 64 * 1024; // 64 KiB

#[async_trait]
impl Codec for FileTransferCodec {
    type Protocol = FileTransferProtocol;
    type Request = ProtocolRequest;
    type Response = ProtocolResponse;

    async fn read_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        // Read length prefix (4 bytes)
        let mut len_bytes = [0u8; 4];
        io.read_exact(&mut len_bytes).await?;
        let len = u32::from_be_bytes(len_bytes) as usize;
        if len > MAX_FRAME_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("request frame too large: {} > {}", len, MAX_FRAME_SIZE),
            ));
        }

        // Read data
        let mut buffer = vec![0u8; len];
        io.read_exact(&mut buffer).await?;

        // Deserialize
        let (request, _) = bincode::decode_from_slice(&buffer, config::standard())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Additional budget checks per variant
        match &request {
            ProtocolRequest::HandshakeRequest { .. } => {
                if len > MAX_HANDSHAKE_SIZE {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("handshake frame too large: {} > {}", len, MAX_HANDSHAKE_SIZE),
                    ));
                }
            }
            ProtocolRequest::FileChunk { data, .. } => {
                if data.len() > MAX_FRAME_SIZE {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("chunk data too large: {} > {}", data.len(), MAX_FRAME_SIZE),
                    ));
                }
            }
            ProtocolRequest::CancelTransfer { .. } => {}
        }

        Ok(request)
    }

    async fn read_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        // Read length prefix (4 bytes)
        let mut len_bytes = [0u8; 4];
        io.read_exact(&mut len_bytes).await?;
        let len = u32::from_be_bytes(len_bytes) as usize;
        if len > MAX_FRAME_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("response frame too large: {} > {}", len, MAX_FRAME_SIZE),
            ));
        }

        // Read data
        let mut buffer = vec![0u8; len];
        io.read_exact(&mut buffer).await?;

        // Deserialize
        let (response, _) = bincode::decode_from_slice(&buffer, config::standard())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(response)
    }

    async fn write_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        // Serialize
        let data = bincode::encode_to_vec(req, config::standard())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Write length prefix
        let len = data.len() as u32;
        io.write_all(&len.to_be_bytes()).await?;

        // Write data
        io.write_all(&data).await?;
        io.flush().await?;

        Ok(())
    }

    async fn write_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        res: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        // Serialize
        let data = bincode::encode_to_vec(res, config::standard())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Write length prefix
        let len = data.len() as u32;
        io.write_all(&len.to_be_bytes()).await?;

        // Write data
        io.write_all(&data).await?;
        io.flush().await?;

        Ok(())
    }
} 