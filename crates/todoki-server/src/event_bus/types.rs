use chrono::{DateTime, Utc};
use gotcha::Schematic;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Core event structure for the Event Bus
#[derive(Debug, Clone, Serialize, Deserialize, Schematic)]
pub struct Event {
    /// Global monotonic sequence number (cursor)
    /// Used for incremental consumption and replay
    pub cursor: i64,

    /// Event kind (semantic type)
    /// Examples: "task.created", "agent.started", "artifact.github_pr_opened"
    pub kind: String,

    /// Timestamp (ISO 8601)
    pub time: DateTime<Utc>,

    /// Agent ID that emitted this event
    /// Use Uuid::nil() for system events
    pub agent_id: Uuid,

    /// Optional session ID (for agent execution events)
    pub session_id: Option<Uuid>,

    /// Optional task ID (for task-related events)
    pub task_id: Option<Uuid>,

    /// Event-specific data (JSON)
    pub data: serde_json::Value,
}

impl Event {
    /// Create a new event (cursor will be assigned by store)
    pub fn new(
        kind: impl Into<String>,
        agent_id: Uuid,
        data: serde_json::Value,
    ) -> Self {
        Self {
            cursor: 0, // Will be assigned by store
            kind: kind.into(),
            time: Utc::now(),
            agent_id,
            session_id: None,
            task_id: None,
            data,
        }
    }

    /// Create a task-related event
    pub fn with_task(
        kind: impl Into<String>,
        agent_id: Uuid,
        task_id: Uuid,
        data: serde_json::Value,
    ) -> Self {
        Self {
            cursor: 0,
            kind: kind.into(),
            time: Utc::now(),
            agent_id,
            session_id: None,
            task_id: Some(task_id),
            data,
        }
    }

    /// Create a session-related event
    pub fn with_session(
        kind: impl Into<String>,
        agent_id: Uuid,
        session_id: Uuid,
        data: serde_json::Value,
    ) -> Self {
        Self {
            cursor: 0,
            kind: kind.into(),
            time: Utc::now(),
            agent_id,
            session_id: Some(session_id),
            task_id: None,
            data,
        }
    }
}

