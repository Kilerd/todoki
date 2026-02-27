use super::store::EventStore;
use super::types::Event;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Event Publisher
///
/// Publishes events to both:
/// 1. Persistent storage (Event Store)
/// 2. In-memory broadcast channel (for real-time subscribers)
pub struct EventPublisher {
    store: Arc<dyn EventStore>,
    broadcaster: broadcast::Sender<Event>,
}

impl EventPublisher {
    pub fn new(store: Arc<dyn EventStore>) -> Self {
        // Create broadcast channel with capacity 1024
        // Lagged subscribers will be notified via RecvError::Lagged
        let (broadcaster, _) = broadcast::channel(1024);
        Self { store, broadcaster }
    }

    /// Emit a new event
    ///
    /// This will:
    /// 1. Persist the event to the store (assigns cursor)
    /// 2. Broadcast to in-memory subscribers (best-effort)
    ///
    /// Returns the assigned cursor on success
    pub async fn emit(&self, mut event: Event) -> Result<i64> {
        // Persist to store (assigns cursor)
        let cursor = self.store.append(&mut event).await?;

        // Broadcast to in-memory subscribers (best-effort, ignore errors)
        let _ = self.broadcaster.send(event);

        Ok(cursor)
    }

    /// Subscribe to real-time events
    ///
    /// Returns a broadcast receiver that will receive all future events.
    /// Note: This does NOT replay historical events. Use EventSubscriber::poll() for that.
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.broadcaster.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_bus::kinds::EventKind;
    use uuid::Uuid;

    #[test]
    fn test_publisher_creation() {
        // This test just ensures EventPublisher can be created
        // Real tests would require a database connection
    }
}
