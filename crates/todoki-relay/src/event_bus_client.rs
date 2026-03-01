use serde::Serialize;
use serde_json::Value;
use todoki_protocol::event_bus::{BuiltinEvent, Event, EventMessage};
use uuid::Uuid;

/// Client for emitting events to event-bus via HTTP API
#[derive(Clone)]
pub struct EventBusClient {
    http_client: reqwest::Client,
    base_url: String,
    token: String,
    agent_id: Uuid,
}

impl EventBusClient {
    pub fn new(server_url: &str, token: &str, agent_id: Uuid) -> Self {
        // Extract base URL: remove WebSocket path and convert protocol
        let base_url = server_url
            .trim_end_matches("/ws/relay")
            .trim_end_matches("/ws/relays")
            .trim_end_matches("/ws/event-bus")
            .replace("wss://", "https://")
            .replace("ws://", "http://");

        Self {
            http_client: reqwest::Client::new(),
            base_url,
            token: token.to_string(),
            agent_id,
        }
    }

    /// Emit an event to event-bus
    pub async fn emit(&self, kind: &str, data: Value) -> Result<i64, EventBusError> {
        let url = format!("{}/api/event-bus/emit", self.base_url);

        let payload = serde_json::json!({
            "kind": kind,
            "data": data,
            "agent_id": self.agent_id,
        });

        let resp = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&payload)
            .send()
            .await
            .map_err(|e| EventBusError::Network(e.to_string()))?;

        if resp.status().is_success() {
            let cursor = resp
                .json::<i64>()
                .await
                .map_err(|e| EventBusError::Parse(e.to_string()))?;
            tracing::debug!(kind = %kind, cursor = cursor, "emitted event to event-bus");
            Ok(cursor)
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::warn!(kind = %kind, status = %status, body = %body, "failed to emit event");
            Err(EventBusError::Server(status.as_u16(), body))
        }
    }

    /// Emit an event, logging errors but not propagating them
    pub async fn emit_fire_and_forget(&self, kind: &str, data: Value) {
        if let Err(e) = self.emit(kind, data).await {
            tracing::warn!(kind = %kind, error = %e, "failed to emit event to event-bus");
        }
    }

    /// Emit a typed builtin event
    pub async fn emit_builtin(&self, event: BuiltinEvent) -> Result<i64, EventBusError> {
        let message = EventMessage {
            event: Event::Builtin(event),
            agent_id: self.agent_id.to_string(),
        };
        self.emit_message(&message).await
    }

    /// Emit a typed builtin event, logging errors but not propagating them
    pub async fn emit_builtin_fire_and_forget(&self, event: BuiltinEvent) {
        if let Err(e) = self.emit_builtin(event).await {
            tracing::warn!(error = %e, "failed to emit builtin event to event-bus");
        }
    }

    /// Internal: emit any serializable message
    async fn emit_message<T: Serialize>(&self, message: &T) -> Result<i64, EventBusError> {
        let url = format!("{}/api/event-bus/emit", self.base_url);

        let resp = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .json(message)
            .send()
            .await
            .map_err(|e| EventBusError::Network(e.to_string()))?;

        if resp.status().is_success() {
            let cursor = resp
                .json::<i64>()
                .await
                .map_err(|e| EventBusError::Parse(e.to_string()))?;
            tracing::debug!(cursor = cursor, "emitted event to event-bus");
            Ok(cursor)
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::warn!(status = %status, body = %body, "failed to emit event");
            Err(EventBusError::Server(status.as_u16(), body))
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EventBusError {
    #[error("network error: {0}")]
    Network(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("server error {0}: {1}")]
    Server(u16, String),
}
