use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

/// Event broadcasted to subscribers
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentStreamEvent {
    pub agent_id: Uuid,
    pub session_id: Uuid,
    /// Database id (for ordering)
    pub id: i64,
    /// Original sequence from relay
    pub seq: i64,
    pub ts: String,
    pub stream: String,
    pub message: String,
}

/// Manages broadcast channels for agent events
#[derive(Clone)]
pub struct AgentBroadcaster {
    /// agent_id -> broadcast sender
    channels: Arc<RwLock<HashMap<Uuid, broadcast::Sender<AgentStreamEvent>>>>,
}

impl AgentBroadcaster {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Subscribe to an agent's event stream
    pub async fn subscribe(&self, agent_id: Uuid) -> broadcast::Receiver<AgentStreamEvent> {
        let mut channels = self.channels.write().await;

        if let Some(tx) = channels.get(&agent_id) {
            tx.subscribe()
        } else {
            // Create new channel with buffer size 256
            let (tx, rx) = broadcast::channel(256);
            channels.insert(agent_id, tx);
            rx
        }
    }

    /// Broadcast an event to all subscribers of an agent
    pub async fn broadcast(&self, event: AgentStreamEvent) {
        let channels = self.channels.read().await;

        if let Some(tx) = channels.get(&event.agent_id) {
            // Ignore send errors (no subscribers)
            let _ = tx.send(event);
        }
    }

    /// Get subscriber count for an agent (for debugging)
    pub async fn subscriber_count(&self, agent_id: Uuid) -> usize {
        let channels = self.channels.read().await;
        channels
            .get(&agent_id)
            .map(|tx| tx.receiver_count())
            .unwrap_or(0)
    }

    /// Clean up channels with no subscribers
    pub async fn cleanup_empty_channels(&self) {
        let mut channels = self.channels.write().await;
        channels.retain(|_, tx| tx.receiver_count() > 0);
    }
}

impl Default for AgentBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}
