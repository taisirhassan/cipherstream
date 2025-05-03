use std::io;
use std::time::Duration;
use libp2p::PeerId;
use futures::io::{AsyncRead, AsyncWrite};
use futures::AsyncReadExt;
use futures::AsyncWriteExt;
use async_trait::async_trait;
use crate::file_transfer::types::{ProtocolRequest, ProtocolResponse};
use libp2p::request_response;

// Protocol name for our file transfer protocol
#[derive(Debug, Clone)]
pub struct FileTransferProtocol(String);

impl FileTransferProtocol {
    pub fn new() -> Self {
        Self("/cipherstream/file-transfer/1.0.0".to_string())
    }
}

impl AsRef<str> for FileTransferProtocol {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// Codec implementation for encoding/decoding requests and responses
#[derive(Clone, Default)]
pub struct FileTransferCodec;

#[async_trait]
impl request_response::Codec for FileTransferCodec {
    type Protocol = FileTransferProtocol;
    type Request = ProtocolRequest;
    type Response = ProtocolResponse;

    async fn read_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut buffer = vec![0u8; 4]; // Length prefix
        io.read_exact(&mut buffer).await?;
        let length = u32::from_be_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]) as usize;

        buffer.resize(length, 0);
        io.read_exact(&mut buffer).await?;
        
        serde_json::from_slice(&buffer)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn read_response<T>(&mut self, _: &Self::Protocol, io: &mut T) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut buffer = vec![0u8; 4]; // Length prefix
        io.read_exact(&mut buffer).await?;
        let length = u32::from_be_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]) as usize;

        buffer.resize(length, 0);
        io.read_exact(&mut buffer).await?;
        
        serde_json::from_slice(&buffer)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn write_request<T>(&mut self, _: &Self::Protocol, io: &mut T, req: Self::Request) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let buffer = serde_json::to_vec(&req)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        
        let len = (buffer.len() as u32).to_be_bytes();
        io.write_all(&len).await?;
        io.write_all(&buffer).await?;
        io.flush().await?;
        
        Ok(())
    }

    async fn write_response<T>(&mut self, _: &Self::Protocol, io: &mut T, res: Self::Response) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let buffer = serde_json::to_vec(&res)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        
        let len = (buffer.len() as u32).to_be_bytes();
        io.write_all(&len).await?;
        io.write_all(&buffer).await?;
        io.flush().await?;
        
        Ok(())
    }
}

// Export the types we need from libp2p::request_response
pub use request_response::{
    Behaviour,
    Event,
    Message,
    ResponseChannel,
    OutboundFailure,
    OutboundRequestId
};

// Create a request-response behavior for file transfers
pub fn create_request_response() -> Behaviour<FileTransferCodec> {
    let protocols = vec![(
        FileTransferProtocol::new(), 
        request_response::ProtocolSupport::Full
    )];
    
    let cfg = request_response::Config::default()
        .with_request_timeout(Duration::from_secs(60))
        .with_max_concurrent_streams(10); // Allow multiple concurrent transfers
    
    Behaviour::new(protocols, cfg)
}

// Helper function to send a file request
pub fn send_file_request(
    request_response: &mut Behaviour<FileTransferCodec>,
    peer_id: PeerId, 
    request: ProtocolRequest
) -> OutboundRequestId {
    request_response.send_request(&peer_id, request)
}

// Helper function to send a file response
pub fn send_file_response(
    request_response: &mut Behaviour<FileTransferCodec>, 
    channel: ResponseChannel<ProtocolResponse>, 
    response: ProtocolResponse
) -> Result<(), OutboundFailure> {
    request_response.send_response(channel, response)
        .map_err(|_| OutboundFailure::ConnectionClosed)
} 