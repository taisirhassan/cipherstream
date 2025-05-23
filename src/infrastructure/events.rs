use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use async_trait::async_trait;
use crate::core::{
    domain::DomainEvent,
    traits::{EventHandler, EventPublisher, DomainResult},
};

/// In-memory event publisher for testing and development
pub struct InMemoryEventPublisher {
    handlers: Arc<RwLock<Vec<Box<dyn EventHandler>>>>,
    event_log: Arc<RwLock<Vec<DomainEvent>>>,
}

impl InMemoryEventPublisher {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(Vec::new())),
            event_log: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get all events that have been published (for testing)
    pub async fn get_events(&self) -> Vec<DomainEvent> {
        self.event_log.read().await.clone()
    }

    /// Clear the event log
    pub async fn clear_events(&self) {
        self.event_log.write().await.clear();
    }
}

#[async_trait]
impl EventPublisher for InMemoryEventPublisher {
    async fn publish(&self, event: DomainEvent) -> DomainResult<()> {
        // Log the event
        self.event_log.write().await.push(event.clone());

        // Notify all handlers
        let handlers = self.handlers.read().await;
        for handler in handlers.iter() {
            if let Err(e) = handler.handle_event(event.clone()).await {
                eprintln!("Error in event handler: {}", e);
            }
        }

        Ok(())
    }

    fn subscribe(&self, handler: Box<dyn EventHandler>) -> DomainResult<()> {
        // Note: This is blocking, but we're in a sync context
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.handlers.write().await.push(handler);
            })
        });
        Ok(())
    }
}

/// Async event publisher using channels for better performance
pub struct ChannelEventPublisher {
    event_tx: mpsc::UnboundedSender<DomainEvent>,
    handlers: Arc<RwLock<Vec<Box<dyn EventHandler>>>>,
}

impl ChannelEventPublisher {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<DomainEvent>) {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        let publisher = Self {
            event_tx,
            handlers: Arc::new(RwLock::new(Vec::new())),
        };

        (publisher, event_rx)
    }

    /// Start the event processing loop
    pub async fn start_processing(
        mut event_rx: mpsc::UnboundedReceiver<DomainEvent>,
        handlers: Arc<RwLock<Vec<Box<dyn EventHandler>>>>,
    ) {
        while let Some(event) = event_rx.recv().await {
            let handlers_guard = handlers.read().await;
            for handler in handlers_guard.iter() {
                if let Err(e) = handler.handle_event(event.clone()).await {
                    eprintln!("Error in event handler: {}", e);
                }
            }
        }
    }
}

#[async_trait]
impl EventPublisher for ChannelEventPublisher {
    async fn publish(&self, event: DomainEvent) -> DomainResult<()> {
        self.event_tx.send(event)
            .map_err(|e| format!("Failed to publish event: {}", e).into())
    }

    fn subscribe(&self, handler: Box<dyn EventHandler>) -> DomainResult<()> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.handlers.write().await.push(handler);
            })
        });
        Ok(())
    }
}

/// Event handler for logging domain events
pub struct LoggingEventHandler;

#[async_trait]
impl EventHandler for LoggingEventHandler {
    async fn handle_event(&self, event: DomainEvent) -> DomainResult<()> {
        match &event {
            DomainEvent::PeerDiscovered { peer } => {
                println!("🔍 Peer discovered: {} with {} addresses", 
                    peer.id.as_str(), peer.addresses.len());
            }
            DomainEvent::PeerConnected { peer_id } => {
                println!("🔗 Peer connected: {}", peer_id.as_str());
            }
            DomainEvent::PeerDisconnected { peer_id } => {
                println!("❌ Peer disconnected: {}", peer_id.as_str());
            }
            DomainEvent::TransferStarted { transfer } => {
                println!("📤 Transfer started: {} -> {}", 
                    transfer.sender.as_str(), transfer.receiver.as_str());
            }
            DomainEvent::TransferProgress { transfer_id, progress } => {
                println!("📊 Transfer progress {}: {:.2}%", 
                    transfer_id.as_str(), progress.percentage);
            }
            DomainEvent::TransferCompleted { transfer_id } => {
                println!("✅ Transfer completed: {}", transfer_id.as_str());
            }
            DomainEvent::TransferFailed { transfer_id, reason } => {
                println!("❌ Transfer failed {}: {}", transfer_id.as_str(), reason);
            }
            DomainEvent::ChunkReceived { transfer_id, chunk } => {
                println!("📦 Chunk {} received for transfer {}", 
                    chunk.index, transfer_id.as_str());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::domain::PeerId;
    

    #[tokio::test]
    async fn test_in_memory_event_publisher() {
        let publisher = InMemoryEventPublisher::new();
        
        let peer_id = PeerId::new("test-peer-id".to_string());
        let event = DomainEvent::PeerConnected { peer_id };
        
        publisher.publish(event.clone()).await.unwrap();
        
        let events = publisher.get_events().await;
        assert_eq!(events.len(), 1);
        
        match &events[0] {
            DomainEvent::PeerConnected { peer_id } => {
                assert_eq!(peer_id.as_str(), "test-peer-id");
            }
            _ => panic!("Unexpected event type"),
        }
    }

    #[tokio::test]
    async fn test_channel_event_publisher() {
        let (publisher, mut event_rx) = ChannelEventPublisher::new();
        
        let peer_id = PeerId::new("test-peer-id".to_string());
        let event = DomainEvent::PeerConnected { peer_id };
        
        publisher.publish(event.clone()).await.unwrap();
        
        let received_event = event_rx.recv().await.unwrap();
        match received_event {
            DomainEvent::PeerConnected { peer_id } => {
                assert_eq!(peer_id.as_str(), "test-peer-id");
            }
            _ => panic!("Unexpected event type"),
        }
    }
} 