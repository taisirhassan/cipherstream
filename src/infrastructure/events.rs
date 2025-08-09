use crate::core::{
    domain::DomainEvent,
    traits::{DomainResult, EventHandler, EventPublisher},
};
use async_trait::async_trait;
use futures::future::join_all;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::{error, info};

/// In-memory event publisher for testing and development
pub struct InMemoryEventPublisher {
    handlers: Arc<RwLock<Vec<Arc<dyn EventHandler>>>>,
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

impl Default for InMemoryEventPublisher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventPublisher for InMemoryEventPublisher {
    async fn publish(&self, event: DomainEvent) -> DomainResult<()> {
        // Log the event
        self.event_log.write().await.push(event.clone());

        // Notify all handlers. Clone Arcs first to avoid holding the lock across await.
        let handlers_snapshot = {
            let handlers = self.handlers.read().await;
            handlers.clone()
        };
        let futures = handlers_snapshot.into_iter().map(|h| {
            let ev = event.clone();
            async move { h.handle_event(ev).await }
        });
        let results = join_all(futures).await;
        for res in results {
            if let Err(e) = res {
                error!("Error in event handler: {}", e);
            }
        }

        Ok(())
    }

    fn subscribe(&self, handler: Box<dyn EventHandler>) -> DomainResult<()> {
        // Note: This is blocking, but we're in a sync context
        let handler_arc: Arc<dyn EventHandler> = Arc::from(handler);
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.handlers.write().await.push(handler_arc);
            })
        });
        Ok(())
    }
}

/// Async event publisher using channels for better performance
pub struct ChannelEventPublisher {
    event_tx: mpsc::UnboundedSender<DomainEvent>,
    handlers: Arc<RwLock<Vec<Arc<dyn EventHandler>>>>,
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
        handlers: Arc<RwLock<Vec<Arc<dyn EventHandler>>>>,
    ) {
        while let Some(event) = event_rx.recv().await {
            let snapshot = {
                let handlers_guard = handlers.read().await;
                handlers_guard.clone()
            };
            let futures = snapshot.into_iter().map(|h| {
                let ev = event.clone();
                async move { h.handle_event(ev).await }
            });
            let results = join_all(futures).await;
            for res in results {
                if let Err(e) = res {
                    error!("Error in event handler: {}", e);
                }
            }
        }
    }
}

#[async_trait]
impl EventPublisher for ChannelEventPublisher {
    async fn publish(&self, event: DomainEvent) -> DomainResult<()> {
        self.event_tx
            .send(event)
            .map_err(|e| format!("Failed to publish event: {}", e).into())
    }

    fn subscribe(&self, handler: Box<dyn EventHandler>) -> DomainResult<()> {
        let handler_arc: Arc<dyn EventHandler> = Arc::from(handler);
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.handlers.write().await.push(handler_arc);
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
                info!(
                    "Peer discovered: {} with {} addresses",
                    peer.id.as_str(),
                    peer.addresses.len()
                );
            }
            DomainEvent::PeerConnected { peer_id } => {
                info!("Peer connected: {}", peer_id.as_str());
            }
            DomainEvent::PeerDisconnected { peer_id } => {
                info!("Peer disconnected: {}", peer_id.as_str());
            }
            DomainEvent::TransferStarted { transfer } => {
                info!(
                    "Transfer started: {} -> {}",
                    transfer.sender.as_str(),
                    transfer.receiver.as_str()
                );
            }
            DomainEvent::TransferProgress {
                transfer_id,
                progress,
            } => {
                info!(
                    "Transfer progress {}: {:.2}%",
                    transfer_id.as_str(),
                    progress.percentage
                );
            }
            DomainEvent::TransferCompleted { transfer_id } => {
                info!("Transfer completed: {}", transfer_id.as_str());
            }
            DomainEvent::TransferFailed {
                transfer_id,
                reason,
            } => {
                error!("Transfer failed {}: {}", transfer_id.as_str(), reason);
            }
            DomainEvent::ChunkReceived { transfer_id, chunk } => {
                info!(
                    "Chunk {} received for transfer {}",
                    chunk.index,
                    transfer_id.as_str()
                );
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
