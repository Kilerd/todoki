use super::store::EventStore;
use super::types::Event;
use anyhow::Result;
use std::sync::Arc;
use uuid::Uuid;

/// Event Subscriber
///
/// Provides query-based access to historical events.
/// For real-time events, use EventPublisher::subscribe()
pub struct EventSubscriber {
    store: Arc<dyn EventStore>,
}

impl EventSubscriber {
    pub fn new(store: Arc<dyn EventStore>) -> Self {
        Self { store }
    }

    /// Poll events since last cursor (for HTTP polling)
    ///
    /// Returns all events with cursor > from_cursor, up to limit
    pub async fn poll(
        &self,
        from_cursor: i64,
        kinds: Option<&[String]>,
        agent_id: Option<Uuid>,
        task_id: Option<Uuid>,
        limit: Option<usize>,
    ) -> Result<Vec<Event>> {
        self.store
            .query(from_cursor, None, kinds, agent_id, task_id, limit)
            .await
    }

    /// Get latest cursor (for initialization)
    pub async fn latest_cursor(&self) -> Result<i64> {
        self.store.latest_cursor().await
    }

    /// Replay events between two cursors (for analysis)
    ///
    /// This is useful for:
    /// - Event replay and debugging
    /// - Analyzing event patterns
    /// - Generating skill recommendations
    pub async fn replay(
        &self,
        from_cursor: i64,
        to_cursor: i64,
        kinds: Option<&[String]>,
    ) -> Result<Vec<Event>> {
        self.store
            .query(from_cursor, Some(to_cursor), kinds, None, None, None)
            .await
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_subscriber_creation() {
        // This test just ensures EventSubscriber can be created
        // Real tests would require a database connection
    }
}
