use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error};

/// Event structure (matches server-side Event)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub cursor: i64,
    pub kind: String,
    pub time: String,
    pub agent_id: String,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct EventQueryResponse {
    events: Vec<Event>,
    next_cursor: i64,
}

/// Event Poller
///
/// Allows relay-side agents to poll events from the server.
/// This is a supplementary mechanism to the push-based Orchestrator.
///
/// Use cases:
/// - Standalone agents that need to query historical events
/// - Agents that want to check for missed events during reconnection
pub struct EventPoller {
    agent_id: String,
    last_cursor: Arc<RwLock<i64>>,
    server_url: String,
    token: String,
    kinds: Vec<String>,
}

impl EventPoller {
    pub fn new(
        agent_id: String,
        server_url: String,
        token: String,
        kinds: Vec<String>,
    ) -> Self {
        Self {
            agent_id,
            last_cursor: Arc::new(RwLock::new(0)),
            server_url,
            token,
            kinds,
        }
    }

    /// Initialize cursor from server's latest cursor
    pub async fn init_cursor(&self) -> Result<()> {
        let url = format!("{}/api/event-bus/latest", self.server_url);

        debug!(agent_id = %self.agent_id, "fetching latest cursor");

        let resp = reqwest::Client::new()
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!(
                "failed to get latest cursor: {}",
                resp.status()
            ));
        }

        let cursor: i64 = resp.json().await?;
        *self.last_cursor.write().await = cursor;

        debug!(agent_id = %self.agent_id, cursor = cursor, "cursor initialized");
        Ok(())
    }

    /// Poll once for new events
    pub async fn poll_once(&self) -> Result<Vec<Event>> {
        let cursor = *self.last_cursor.read().await;

        let url = format!(
            "{}/api/event-bus?cursor={}&kinds={}",
            self.server_url,
            cursor,
            self.kinds.join(",")
        );

        debug!(agent_id = %self.agent_id, cursor = cursor, "polling events");

        let resp = reqwest::Client::new()
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!(
                "event poll failed: {}",
                resp.status()
            ));
        }

        let data: EventQueryResponse = resp.json().await?;

        if !data.events.is_empty() {
            *self.last_cursor.write().await = data.next_cursor;
            debug!(
                agent_id = %self.agent_id,
                count = data.events.len(),
                next_cursor = data.next_cursor,
                "polled events"
            );
        }

        Ok(data.events)
    }

    /// Start continuous polling (spawns background task)
    pub async fn start_polling<F>(&self, interval_secs: u64, handler: F)
    where
        F: Fn(Event) + Send + Sync + 'static,
    {
        let agent_id = self.agent_id.clone();
        let server_url = self.server_url.clone();
        let token = self.token.clone();
        let kinds = self.kinds.clone();
        let last_cursor = self.last_cursor.clone();

        tokio::spawn(async move {
            debug!(agent_id = %agent_id, "event poller started");

            let poller = EventPoller {
                agent_id: agent_id.clone(),
                last_cursor,
                server_url,
                token,
                kinds,
            };

            loop {
                match poller.poll_once().await {
                    Ok(events) => {
                        for event in events {
                            handler(event);
                        }
                    }
                    Err(e) => {
                        error!(agent_id = %agent_id, error = %e, "event polling error");
                    }
                }

                tokio::time::sleep(tokio::time::Duration::from_secs(interval_secs)).await;
            }
        });
    }
}
