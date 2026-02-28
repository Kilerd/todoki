mod broadcaster;
mod manager;
mod request_tracker;

use std::collections::HashMap;

pub use broadcaster::{AgentBroadcaster, AgentStreamEvent};
pub use manager::RelayManager;
pub use request_tracker::RequestTracker;

use gotcha::Schematic;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Re-export shared protocol types from todoki-protocol
pub use todoki_protocol::{PermissionOutcome, RelayRole, RelayToServer, ServerToRelay};

// ============================================================================
// Public types
// ============================================================================

/// Public info about a relay (for API responses)
#[derive(Debug, Clone, Serialize, Deserialize, Schematic)]
pub struct RelayInfo {
    pub relay_id: String,
    pub name: String,
    pub role: String,
    pub safe_paths: Vec<String>,
    pub labels: HashMap<String, String>,
    pub projects: Vec<Uuid>,
    pub setup_script: Option<String>,
    pub connected_at: i64,
    pub active_session_count: usize,
}

/// Agent output event (for broadcasting to clients)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutputEvent {
    pub agent_id: String,
    pub session_id: String,
    pub seq: i64,
    pub ts: i64,
    pub stream: String,
    pub message: String,
}

/// Session status event (for broadcasting to clients)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStatusEvent {
    pub session_id: String,
    pub status: String,
    pub exit_code: Option<i32>,
}
