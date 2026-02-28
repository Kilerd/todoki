use super::{EventPublisher, Event};
use crate::db::DatabaseService;
use crate::models::agent::{Agent, AgentStatus, AgentSession, CreateAgentSession};
use crate::relay::RelayManager;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;
use conservator::Creatable;

/// Event Orchestrator
///
/// Background service that monitors events and triggers subscribed agents
pub struct EventOrchestrator {
    publisher: Arc<EventPublisher>,
    db: Arc<DatabaseService>,
    relay_manager: Arc<RelayManager>,
    active: Arc<RwLock<bool>>,
}

impl EventOrchestrator {
    pub fn new(
        publisher: Arc<EventPublisher>,
        db: Arc<DatabaseService>,
        relay_manager: Arc<RelayManager>,
    ) -> Self {
        Self {
            publisher,
            db,
            relay_manager,
            active: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the orchestrator (spawns background task)
    pub async fn start(&self) -> Result<()> {
        if *self.active.read().await {
            warn!("Event orchestrator already running");
            return Ok(());
        }

        *self.active.write().await = true;

        let publisher = self.publisher.clone();
        let db = self.db.clone();
        let relay_manager = self.relay_manager.clone();
        let active = self.active.clone();

        tokio::spawn(async move {
            info!("Event orchestrator started");

            let mut event_rx = publisher.subscribe();

            while *active.read().await {
                match event_rx.recv().await {
                    Ok(event) => {
                        if let Err(e) = Self::handle_event(&event, &db, &relay_manager, &publisher).await {
                            error!("Failed to handle event {}: {}", event.cursor, e);
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!("Event orchestrator lagged by {} events", n);
                    }
                    Err(e) => {
                        error!("Event receive error: {}", e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                }
            }

            info!("Event orchestrator stopped");
        });

        Ok(())
    }

    /// Stop the orchestrator
    pub async fn stop(&self) {
        *self.active.write().await = false;
    }

    /// Handle a single event (check subscriptions, trigger agents)
    async fn handle_event(
        event: &Event,
        db: &Arc<DatabaseService>,
        relay_manager: &Arc<RelayManager>,
        publisher: &Arc<EventPublisher>,
    ) -> Result<()> {
        // Find agents subscribed to this event kind
        let agents = db.list_agents_by_subscription(&event.kind).await?;

        if agents.is_empty() {
            return Ok(());
        }

        info!(
            "Event {} (kind: {}) matched {} agent(s)",
            event.cursor,
            event.kind,
            agents.len()
        );

        for agent in agents {
            // Check if agent should be triggered
            if !agent.should_trigger(&event.kind) {
                continue;
            }

            // Update last_cursor before triggering (prevent duplicate triggers)
            if let Err(e) = db.update_agent_cursor(agent.id, event.cursor).await {
                error!("Failed to update cursor for agent {}: {}", agent.name, e);
                continue;
            }

            // Trigger agent
            match Self::trigger_agent(&agent, event, db, relay_manager, publisher).await {
                Ok(_) => {
                    info!(
                        "Agent '{}' triggered by event {} (kind: {})",
                        agent.name, event.cursor, event.kind
                    );
                }
                Err(e) => {
                    error!(
                        "Failed to trigger agent '{}' for event {}: {}",
                        agent.name, event.cursor, e
                    );
                }
            }
        }

        Ok(())
    }

    /// Trigger an agent in response to an event
    async fn trigger_agent(
        agent: &Agent,
        event: &Event,
        db: &Arc<DatabaseService>,
        relay_manager: &Arc<RelayManager>,
        publisher: &Arc<EventPublisher>,
    ) -> Result<()> {
        // Check if relay is available
        let relay_id = agent.relay_id.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Agent '{}' has no relay assigned", agent.name)
        })?;

        if !relay_manager.is_connected(relay_id).await {
            return Err(anyhow::anyhow!("Relay '{}' is not connected", relay_id));
        }

        // Use CreateAgentSession to create a session record
        let create_session = CreateAgentSession {
            agent_id: agent.id,
            relay_id: Some(relay_id.clone()),
        };

        // Insert session and get the created session back
        let pool = db.pool();
        let session_uuid = create_session
            .insert::<AgentSession>()
            .returning_pk(&*pool)
            .await?;

        // Update agent status
        db.update_agent_status(agent.id, AgentStatus::Running).await?;

        // Register active session
        relay_manager
            .add_active_session(relay_id, &session_uuid.to_string())
            .await;

        // Build spawn request
        let workdir = if agent.workdir.is_empty() {
            format!("/tmp/todoki-agent-{}", agent.id)
        } else {
            agent.workdir.clone()
        };

        let mut env: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        env.insert("TRIGGER_EVENT_KIND".to_string(), event.kind.clone());
        env.insert("TRIGGER_EVENT_CURSOR".to_string(), event.cursor.to_string());
        env.insert("TRIGGER_EVENT_DATA".to_string(), event.data.to_string());

        if let Some(task_id) = event.task_id {
            env.insert("TASK_ID".to_string(), task_id.to_string());
        }

        let data = serde_json::json!({
            "agent_id": agent.id.to_string(),
            "session_id": session_uuid.to_string(),
            "workdir": workdir,
            "command": agent.command,
            "args": agent.args_vec(),
            "env": env,
        });

        // Emit relay.spawn_requested event via Event Bus (fire-and-forget for orchestrator)
        let request_id = Uuid::new_v4().to_string();
        relay_manager
            .emit_relay_command(
                publisher.as_ref(),
                relay_id,
                super::kinds::EventKind::RELAY_SPAWN_REQUESTED,
                request_id,
                data,
            )
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::agent::{Agent, AgentRole, ExecutionMode};
    use chrono::Utc;

    #[test]
    fn test_agent_subscription_matching() {
        let mut agent = Agent {
            id: Uuid::new_v4(),
            name: "test-agent".to_string(),
            command: "mock-agent".to_string(),
            args: "".to_string(),
            execution_mode: ExecutionMode::Local,
            role: AgentRole::General,
            project_id: Uuid::new_v4(),
            relay_id: Some("test-relay".to_string()),
            subscribed_events: vec!["task.created".to_string(), "agent.*".to_string()],
            auto_trigger: true,
            last_cursor: 0,
            status: AgentStatus::Created,
            workdir: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Test exact match
        assert!(agent.subscribes_to("task.created"));

        // Test wildcard match
        assert!(agent.subscribes_to("agent.requirement_analyzed"));
        assert!(agent.subscribes_to("agent.task_completed"));

        // Test no match
        assert!(!agent.subscribes_to("project.created"));

        // Test should_trigger
        assert!(agent.should_trigger("task.created"));

        // Should not trigger when running
        agent.status = AgentStatus::Running;
        assert!(!agent.should_trigger("task.created"));

        // Should not trigger when auto_trigger is false
        agent.status = AgentStatus::Created;
        agent.auto_trigger = false;
        assert!(!agent.should_trigger("task.created"));

        // Should not trigger for non-subscribed events
        agent.auto_trigger = true;
        assert!(!agent.should_trigger("project.created"));
    }

    #[test]
    fn test_event_kind_prefix_matching() {
        let agent = Agent {
            id: Uuid::new_v4(),
            name: "monitor".to_string(),
            command: "mock-agent".to_string(),
            args: "".to_string(),
            execution_mode: ExecutionMode::Local,
            role: AgentRole::General,
            project_id: Uuid::new_v4(),
            relay_id: None,
            subscribed_events: vec!["task.*".to_string(), "system.relay_*".to_string()],
            auto_trigger: true,
            last_cursor: 0,
            status: AgentStatus::Created,
            workdir: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Test task.* pattern
        assert!(agent.subscribes_to("task.created"));
        assert!(agent.subscribes_to("task.updated"));
        assert!(agent.subscribes_to("task.deleted"));
        assert!(agent.subscribes_to("task.completed"));

        // Test system.relay_* pattern
        assert!(agent.subscribes_to("system.relay_connected"));
        assert!(agent.subscribes_to("system.relay_disconnected"));

        // Test non-matching
        assert!(!agent.subscribes_to("agent.started"));
        assert!(!agent.subscribes_to("project.created"));
        assert!(!agent.subscribes_to("system.error"));
    }

    #[test]
    fn test_wildcard_edge_cases() {
        let agent = Agent {
            id: Uuid::new_v4(),
            name: "wildcard-test".to_string(),
            command: "mock-agent".to_string(),
            args: "".to_string(),
            execution_mode: ExecutionMode::Local,
            role: AgentRole::General,
            project_id: Uuid::new_v4(),
            relay_id: None,
            subscribed_events: vec!["*".to_string()], // Match everything
            auto_trigger: true,
            last_cursor: 0,
            status: AgentStatus::Created,
            workdir: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Wildcard "*" should match any event
        assert!(agent.subscribes_to("task.created"));
        assert!(agent.subscribes_to("agent.started"));
        assert!(agent.subscribes_to("system.error"));
        assert!(agent.subscribes_to("anything"));
    }
}
